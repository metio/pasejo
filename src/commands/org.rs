// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

//! `pasejo org`: organizations, their devices, members, teams, settings,
//! recovery and pending actions.

use std::io::IsTerminal;

use anyhow::Result;
use pasejo_client::Error;
use pasejo_client::actions::Outcome;
use pasejo_protocol::api::{Action, Decision, Enrollment, ErrorCode, Review, UserSummary};
use pasejo_protocol::device::org_fingerprint;
use pasejo_protocol::encoding::{b64, unb64};
use pasejo_protocol::log::{Approval, Body, OrgLog, Principal, RoleChange, Settings};
use pasejo_protocol::{Id, Millis};

use crate::cli::{i18n, prompts};
use crate::models::cli::{
    ApprovalArg, OrgActionsArgs, OrgApproveArgs, OrgCommands, OrgCosignArgs, OrgCreateArgs,
    OrgJoinArgs, OrgRecoveryCommands, OrgReviewCommands, OrgReviewDecideArgs, OrgReviewOpenArgs,
    OrgRevokeArgs, OrgRoleArg, OrgRoleArgs, OrgScimTokenCommands, OrgSettingsArgs, OrgTeamCommands,
    ReviewDecisionArg,
};
use crate::models::configuration::{Configuration, OrgRegistration};
use crate::remote::connect::{self, Remote, parse_id};
use crate::remote::errors::Localize;

pub fn dispatch(command: &OrgCommands, configuration: &Configuration) -> Result<()> {
    match command {
        OrgCommands::Actions(args) => actions(configuration, args),
        OrgCommands::Approve(args) => approve(configuration, args),
        OrgCommands::Cosign(args) => cosign(configuration, args),
        OrgCommands::Create(args) => create(configuration, args),
        OrgCommands::Devices(args) => devices(configuration, args.org.as_deref()),
        OrgCommands::Join(args) => join(configuration, args),
        OrgCommands::Proposals(args) => proposals(configuration, args.org.as_deref()),
        OrgCommands::Recover(args) => crate::commands::recovery::recover(configuration, args),
        OrgCommands::Recovery { command } => match command {
            OrgRecoveryCommands::Set(args) => {
                crate::remote::funnel::validate_recipient(&args.recipient)?;
                let mut org = OrgContext::open(configuration, args.org_selection.org.as_deref())?;
                org.change(Body::RecoverySet {
                    recipient: Some(args.recipient.clone()),
                })
            }
        },
        OrgCommands::Review { command } => match command {
            OrgReviewCommands::Decide(args) => decide_review(configuration, args),
            OrgReviewCommands::List(args) => reviews(configuration, args.org.as_deref()),
            OrgReviewCommands::Open(args) => open_review(configuration, args),
        },
        OrgCommands::Revoke(args) => revoke(configuration, args),
        OrgCommands::Role(args) => role(configuration, args),
        OrgCommands::ScimToken { command } => scim_token(configuration, command),
        OrgCommands::Settings(args) => settings(configuration, args),
        OrgCommands::Stores(args) => {
            crate::commands::remote_stores::list(configuration, args.org.as_deref())
        }
        OrgCommands::Team { command } => team(configuration, command),
    }
}

/// An organization opened for a command: a signed-in client and the
/// verified membership log.
pub struct OrgContext {
    pub remote: Remote,
    pub org: OrgRegistration,
    pub log: OrgLog,
}

impl OrgContext {
    /// # Errors
    ///
    /// Fails when the organization is not registered, the device is not
    /// signed in, or the log does not verify.
    pub fn open(configuration: &Configuration, org: Option<&str>) -> Result<Self> {
        let (server, org) = connect::org(configuration, org)?;
        let remote = Remote::connect(server)?;
        let log = remote.log(org)?;
        Ok(Self {
            remote,
            org: org.clone(),
            log,
        })
    }

    pub const fn id(&self) -> Id {
        self.log.state().org
    }

    /// The server's checkpoint key pinned at the first sign-in.
    ///
    /// # Errors
    ///
    /// Fails when the pinned key is malformed.
    pub fn server_key(&self) -> Result<[u8; 32]> {
        unb64(&self.remote.server.checkpoint_key)
            .ok()
            .and_then(|bytes| bytes.try_into().ok())
            .ok_or_else(|| {
                anyhow::anyhow!(i18n::error_remote_invalid_id(
                    &self.remote.server.checkpoint_key
                ))
            })
    }

    /// Appends `body`, or proposes it when it needs a second admin.
    ///
    /// # Errors
    ///
    /// Fails when the log or the server refuses the change.
    pub fn change(&mut self, body: Body) -> Result<()> {
        let kind = body.kind();
        match self.remote.client.append_fresh(&mut self.log, body.clone()) {
            Ok(_) => {
                self.remote.files.save_log(self.id(), &self.log)?;
                i18n::remote_change_recorded(kind);
                Ok(())
            }
            Err(Error::NeedsSecondAdmin) => {
                let proposal = self.remote.client.propose(&self.log, body).localized()?;
                i18n::remote_change_proposed(kind, &proposal.id.to_string());
                Ok(())
            }
            Err(error) => Err(error).localized(),
        }
    }

    /// The member or enrolling user named `name`: an email address, or
    /// `machine:NAME` for a machine user, which has no email address.
    ///
    /// # Errors
    ///
    /// Fails when nobody in the organization goes by that name.
    pub fn user(&self, name: &str) -> Result<Id> {
        let users = self.remote.client.users(self.id()).localized()?;
        if let Some(user) = users
            .iter()
            .find(|user| user_matches(name, &user.email, &user.name, user.machine))
        {
            return Ok(user.id);
        }
        let enrolling = self
            .remote
            .client
            .enrollments(self.id())
            .unwrap_or_default();
        enrolling
            .iter()
            .find(|enrollment| {
                user_matches(
                    name,
                    &enrollment.user_email,
                    &enrollment.user_name,
                    enrollment.user_email.is_empty(),
                )
            })
            .map(|enrollment| enrollment.device.user)
            .ok_or_else(|| anyhow::anyhow!(i18n::error_remote_unknown_user(name)))
    }

    /// The team named `name`.
    ///
    /// # Errors
    ///
    /// Fails for unknown teams.
    pub fn team(&self, name: &str) -> Result<Id> {
        self.log
            .state()
            .teams
            .iter()
            .find(|(_, team)| team.name.eq_ignore_ascii_case(name))
            .map(|(id, _)| *id)
            .ok_or_else(|| anyhow::anyhow!(i18n::error_remote_unknown_team(name)))
    }

    /// The member named by email address, or else the team named `name`.
    ///
    /// # Errors
    ///
    /// Fails when neither exists.
    pub fn principal(&self, name: &str) -> Result<Principal> {
        if name.contains('@') || name.starts_with(MACHINE_PREFIX) {
            Ok(Principal::User(self.user(name)?))
        } else {
            Ok(Principal::Team(self.team(name)?))
        }
    }

    /// A member's email address, or team's name, for display.
    pub fn describe(&self, principal: &Principal, users: &[UserSummary]) -> String {
        match principal {
            Principal::User(id) => users
                .iter()
                .find(|user| user.id == *id)
                .map_or_else(|| id.to_string(), user_display),
            Principal::Team(id) => self
                .log
                .state()
                .teams
                .get(id)
                .map_or_else(|| id.to_string(), |team| team.name.clone()),
        }
    }

    /// The identities that open this organization's store keys: the global
    /// ones.
    ///
    /// # Errors
    ///
    /// Fails when no identity is configured or one cannot be read.
    pub fn identities(configuration: &Configuration) -> Result<Vec<Box<dyn age::Identity>>> {
        configuration.read_identity_files(
            configuration
                .identities
                .iter()
                .map(|identity| identity.file.clone())
                .collect(),
        )
    }
}

/// How the CLI names a machine user, which has no email address.
pub const MACHINE_PREFIX: &str = "machine:";

/// Whether `name`, as a person types it, names the user with `email` and
/// display name `display`.
fn user_matches(name: &str, email: &str, display: &str, machine: bool) -> bool {
    if let Some(machine_name) = name.strip_prefix(MACHINE_PREFIX) {
        return machine && display == machine_name;
    }
    !email.is_empty() && email.eq_ignore_ascii_case(name)
}

/// A user as the CLI names them: the email address, or `machine:NAME`.
fn user_display(user: &UserSummary) -> String {
    if user.machine {
        format!("{MACHINE_PREFIX}{}", user.name)
    } else {
        user.email.clone()
    }
}

/// The owner of an enrolling device as the CLI names them.
fn enrollment_display(enrollment: &Enrollment) -> String {
    if enrollment.user_email.is_empty() {
        format!("{MACHINE_PREFIX}{}", enrollment.user_name)
    } else {
        enrollment.user_email.clone()
    }
}

/// Compares `expected` with the fingerprint the user confirms: the given
/// `--fingerprint`, or an interactive confirmation.
fn confirm_fingerprint(expected: &str, given: Option<&str>) -> Result<()> {
    let normalize = |text: &str| {
        text.chars()
            .filter(char::is_ascii_hexdigit)
            .collect::<String>()
            .to_ascii_lowercase()
    };
    match given {
        Some(given) if normalize(given) == normalize(expected) => Ok(()),
        Some(_) => anyhow::bail!(i18n::error_remote_fingerprint_mismatch(expected)),
        None if std::io::stdin().is_terminal() => {
            if prompts::get_confirmation_from_user(&i18n::prompt_remote_confirm_fingerprint(
                expected,
            ))? {
                Ok(())
            } else {
                anyhow::bail!(i18n::error_remote_fingerprint_not_confirmed())
            }
        }
        None => anyhow::bail!(i18n::error_remote_fingerprint_required(expected)),
    }
}

fn create(configuration: &Configuration, args: &OrgCreateArgs) -> Result<()> {
    let server = connect::server(configuration, args.server.as_deref())?;
    let remote = Remote::connect(server)?;
    let id = Id::random();
    let log = remote
        .client
        .create_org(id, &args.name, Settings::default())
        .localized()?;
    remote.files.save_log(id, &log)?;
    let mut configuration = configuration.clone();
    let server_url = server.url.clone();
    if let Some(registration) = configuration
        .servers
        .iter_mut()
        .find(|registered| registered.url == server_url)
    {
        registration.orgs.push(OrgRegistration {
            id: id.to_string(),
            name: args.name.clone(),
            genesis: b64(&log.genesis_hash().0),
        });
    }
    configuration.save_configuration()?;
    i18n::remote_org_created(
        &args.name,
        &id.to_string(),
        &org_fingerprint(&log.genesis_hash()),
    );
    Ok(())
}

fn join(configuration: &Configuration, args: &OrgJoinArgs) -> Result<()> {
    let server = connect::server(configuration, Some(&args.url))?;
    let org = parse_id(&args.org)?;
    let remote = Remote::connect(server)?;
    let mut configuration = configuration.clone();
    let registered = server
        .orgs
        .iter()
        .find(|registered| registered.id == org.to_string())
        .cloned();
    let log = if let Some(registration) = &registered {
        remote.log(registration)?
    } else {
        // Only members and enrolling devices may read the log, so the
        // device asks to join before it sees the fingerprint. An
        // enrollment in an organization whose fingerprint does not
        // match stays unused: the organization is never registered.
        enroll(&remote, org)?;
        let log = remote.client.fetch_log(org, None, None).localized()?;
        confirm_fingerprint(
            &org_fingerprint(&log.genesis_hash()),
            args.fingerprint.as_deref(),
        )?;
        remote.files.save_log(org, &log)?;
        let server_url = server.url.clone();
        if let Some(registration) = configuration
            .servers
            .iter_mut()
            .find(|registered| registered.url == server_url)
        {
            registration.orgs.push(OrgRegistration {
                id: org.to_string(),
                name: log.state().name.clone(),
                genesis: b64(&log.genesis_hash().0),
            });
        }
        configuration.save_configuration()?;
        log
    };
    let name = log.state().name.clone();
    if log.state().active_device(remote.client.device.id).is_some() {
        i18n::remote_device_active(&name);
        return Ok(());
    }
    enroll(&remote, org)?;
    i18n::remote_device_pending(
        &name,
        &remote.client.device.id.to_string(),
        &remote.client.device.fingerprint(),
    );
    Ok(())
}

/// Asks the server to enroll this device in `org`; an existing enrollment
/// counts as success.
fn enroll(remote: &Remote, org: Id) -> Result<()> {
    match remote.client.enroll(org) {
        Ok(()) => Ok(()),
        Err(error) if error.is(ErrorCode::Conflict) => Ok(()),
        Err(error) => Err(error).localized(),
    }
}

fn devices(configuration: &Configuration, org: Option<&str>) -> Result<()> {
    let context = OrgContext::open(configuration, org)?;
    let users = context.remote.client.users(context.id()).localized()?;
    let email = |user: Id| {
        users
            .iter()
            .find(|summary| summary.id == user)
            .map_or_else(|| user.to_string(), user_display)
    };
    for (id, device) in &context.log.state().devices {
        if device.active {
            i18n::remote_device_line(
                &id.to_string(),
                &device.record.label,
                &email(device.record.user),
                &device.record.fingerprint(),
                &i18n::remote_device_state_active(),
            );
        }
    }
    // Only admins and approvers see pending enrollments.
    let enrollments: Vec<Enrollment> = context
        .remote
        .client
        .enrollments(context.id())
        .unwrap_or_default();
    for enrollment in enrollments {
        i18n::remote_device_line(
            &enrollment.device.id.to_string(),
            &enrollment.device.label,
            &enrollment_display(&enrollment),
            &enrollment.fingerprint,
            &i18n::remote_device_state_pending(),
        );
    }
    Ok(())
}

fn approve(configuration: &Configuration, args: &OrgApproveArgs) -> Result<()> {
    let mut context = OrgContext::open(configuration, args.org_selection.org.as_deref())?;
    let device = parse_id(&args.device)?;
    let enrollment = context
        .remote
        .client
        .enrollments(context.id())
        .localized()?
        .into_iter()
        .find(|enrollment| enrollment.device.id == device)
        .ok_or_else(|| anyhow::anyhow!(i18n::error_remote_unknown_device(&args.device)))?;
    confirm_fingerprint(&enrollment.fingerprint, args.fingerprint.as_deref())?;
    if !context
        .log
        .state()
        .members
        .contains_key(&enrollment.device.user)
    {
        anyhow::bail!(i18n::error_remote_device_user_not_member(
            &enrollment_display(&enrollment)
        ));
    }
    context.change(Body::DeviceAdd {
        device: enrollment.device,
    })
}

fn revoke(configuration: &Configuration, args: &OrgRevokeArgs) -> Result<()> {
    let mut context = OrgContext::open(configuration, args.org_selection.org.as_deref())?;
    let device = parse_id(&args.device)?;
    if context.log.state().active_device(device).is_none() {
        anyhow::bail!(i18n::error_remote_unknown_device(&args.device));
    }
    context.change(Body::DeviceRevoke { device })
}

fn role(configuration: &Configuration, args: &OrgRoleArgs) -> Result<()> {
    let mut context = OrgContext::open(configuration, args.org_selection.org.as_deref())?;
    let user = context.user(&args.user)?;
    let role = match args.role {
        OrgRoleArg::Admin => RoleChange::Admin,
        OrgRoleArg::Approver => RoleChange::Approver,
        OrgRoleArg::Auditor => RoleChange::Auditor,
        OrgRoleArg::Member => RoleChange::Member,
        OrgRoleArg::None => RoleChange::None,
    };
    context.change(Body::MemberRole { user, role })
}

fn settings(configuration: &Configuration, args: &OrgSettingsArgs) -> Result<()> {
    let mut context = OrgContext::open(configuration, args.org_selection.org.as_deref())?;
    let current = context.log.state().settings;
    let settings = Settings {
        device_approval: args
            .device_approval
            .map_or(current.device_approval, |approval| match approval {
                ApprovalArg::Manual => Approval::Manual,
                ApprovalArg::Automatic => Approval::Automatic,
            }),
        two_admin: args.two_admin.unwrap_or(current.two_admin),
        auto_join: args.auto_join.unwrap_or(current.auto_join),
    };
    if settings == current {
        i18n::remote_settings_unchanged();
        return Ok(());
    }
    if settings.device_approval == Approval::Automatic
        && current.device_approval != Approval::Automatic
    {
        i18n::remote_automatic_approval_warning();
    }
    context.change(Body::OrgSettings { settings })
}

fn team(configuration: &Configuration, command: &OrgTeamCommands) -> Result<()> {
    match command {
        OrgTeamCommands::Create(args) => {
            let mut context = OrgContext::open(configuration, args.org_selection.org.as_deref())?;
            if context.team(&args.name).is_ok() {
                anyhow::bail!(i18n::error_remote_team_exists(&args.name));
            }
            context.change(Body::TeamCreate {
                team: Id::random(),
                name: args.name.clone(),
            })
        }
        OrgTeamCommands::Add(args) | OrgTeamCommands::Remove(args) => {
            let mut context = OrgContext::open(configuration, args.org_selection.org.as_deref())?;
            let team = context.team(&args.team)?;
            let user = context.user(&args.user)?;
            let member = matches!(command, OrgTeamCommands::Add(_));
            context.change(Body::TeamMember { team, user, member })
        }
    }
}

fn proposals(configuration: &Configuration, org: Option<&str>) -> Result<()> {
    let context = OrgContext::open(configuration, org)?;
    let proposals = context.remote.client.proposals(context.id()).localized()?;
    if proposals.is_empty() {
        i18n::remote_no_proposals();
    }
    let devices = &context.log.state().devices;
    for (proposal, entry) in proposals {
        let author = devices.get(&entry.author).map_or_else(
            || entry.author.to_string(),
            |device| device.record.label.clone(),
        );
        i18n::remote_proposal_line(&proposal.id.to_string(), entry.body.kind(), &author);
    }
    Ok(())
}

fn cosign(configuration: &Configuration, args: &OrgCosignArgs) -> Result<()> {
    let mut context = OrgContext::open(configuration, args.org_selection.org.as_deref())?;
    let id = parse_id(&args.proposal)?;
    let (proposal, entry) = context
        .remote
        .client
        .proposals(context.id())
        .localized()?
        .into_iter()
        .find(|(proposal, _)| proposal.id == id)
        .ok_or_else(|| anyhow::anyhow!(i18n::error_remote_unknown_proposal(&args.proposal)))?;
    context
        .remote
        .client
        .cosign_proposal(&mut context.log, &proposal)
        .localized()?;
    context.remote.files.save_log(context.id(), &context.log)?;
    i18n::remote_change_recorded(entry.body.kind());
    Ok(())
}

/// One line describing a pending action.
fn describe_action(context: &OrgContext, action: &Action, users: &[UserSummary]) -> String {
    let user = |id: &Id| context.describe(&Principal::User(*id), users);
    match action {
        Action::ApproveDevice { enrollment } => i18n::remote_action_approve_device(
            &enrollment.device.label,
            &enrollment_display(enrollment),
            &enrollment.fingerprint,
        ),
        Action::JoinMember { email, .. } => i18n::remote_action_join_member(email),
        Action::RemoveMember { user: id } => i18n::remote_action_remove_member(&user(id)),
        Action::SyncTeam { name, .. } => i18n::remote_action_sync_team(name),
        Action::RotateEpoch { store, .. } => i18n::remote_action_rotate_epoch(&store.to_string()),
        Action::WrapKeys { store, devices, .. } => {
            i18n::remote_action_wrap_keys(&store.to_string(), &devices.len().to_string())
        }
        Action::RevokeExpired { store, principal }
        | Action::RevokeGrant {
            store, principal, ..
        } => i18n::remote_action_revoke_grant(
            &context.describe(principal, users),
            &store.to_string(),
        ),
        Action::CosignCheckpoint { seq } => i18n::remote_action_cosign_checkpoint(&seq.to_string()),
    }
}

/// Prints the outcome of every action run.
pub fn report(context: &OrgContext, outcomes: &[Outcome], users: &[UserSummary]) {
    for outcome in outcomes {
        match outcome {
            Outcome::Done(action) => {
                i18n::remote_action_done(&describe_action(context, action, users));
                if let Action::ApproveDevice { enrollment } = action {
                    i18n::remote_device_approved_automatically(&enrollment.device.label);
                }
            }
            Outcome::Skipped(action, reason) => {
                i18n::remote_action_skipped(&describe_action(context, action, users), reason);
            }
            Outcome::Failed(action, error) => i18n::remote_action_failed(
                &describe_action(context, action, users),
                &format!(
                    "{:#}",
                    anyhow::anyhow!(crate::remote::errors::message(error))
                ),
            ),
        }
    }
}

fn actions(configuration: &Configuration, args: &OrgActionsArgs) -> Result<()> {
    let mut context = OrgContext::open(configuration, args.org_selection.org.as_deref())?;
    let users = context.remote.client.users(context.id()).localized()?;
    if args.run {
        let identities = OrgContext::identities(configuration)?;
        let key = context.server_key()?;
        let outcomes = context
            .remote
            .client
            .run_actions(&mut context.log, &identities, &key)
            .localized()?;
        context.remote.files.save_log(context.id(), &context.log)?;
        if outcomes.is_empty() {
            i18n::remote_no_actions();
        }
        report(&context, &outcomes, &users);
        if outcomes
            .iter()
            .any(|outcome| matches!(outcome, Outcome::Failed(..)))
        {
            anyhow::bail!(i18n::error_remote_actions_failed());
        }
        return Ok(());
    }
    let pending = context.remote.client.actions(context.id()).localized()?;
    if pending.is_empty() {
        i18n::remote_no_actions();
    }
    for action in &pending {
        i18n::remote_action_pending(&describe_action(&context, action, &users));
    }
    Ok(())
}

/// The UTC day of `end`, an end-of-day instant as `parser::utc_date` returns
/// it.
fn day(end: Millis) -> String {
    let mut text = crate::commands::audit::rfc3339(end.saturating_sub(1));
    text.truncate(10);
    text
}

/// The local name of a remote store, or its identifier when this
/// installation has not registered it.
fn store_name(configuration: &Configuration, store: Id) -> String {
    configuration
        .stores
        .iter()
        .find(|registration| {
            registration
                .remote
                .as_ref()
                .is_some_and(|remote| remote.store == store.to_string())
        })
        .map_or_else(
            || store.to_string(),
            |registration| registration.name.clone(),
        )
}

fn decision_text(decision: Option<Decision>) -> String {
    match decision {
        Some(Decision::Keep) => i18n::remote_review_decision_keep(),
        Some(Decision::Revoke) => i18n::remote_review_decision_revoke(),
        None => i18n::remote_review_decision_pending(),
    }
}

fn open_review(configuration: &Configuration, args: &OrgReviewOpenArgs) -> Result<()> {
    let registration = crate::commands::store_op::require_store(
        configuration,
        args.store_selection.store.as_ref(),
    )?;
    let Some(location) = &registration.remote else {
        anyhow::bail!(i18n::error_remote_store_required(&registration.name));
    };
    let (_, org, store) = crate::remote::funnel::locate(configuration, location)?;
    let context = OrgContext::open(configuration, Some(&org.id))?;
    let reviewers = args
        .reviewers
        .iter()
        .map(|email| context.user(email))
        .collect::<Result<Vec<_>>>()?;
    let review = context
        .remote
        .client
        .open_review(context.id(), store, reviewers, args.due)
        .localized()?;
    i18n::remote_review_opened(&review.id.to_string(), &registration.name, &day(review.due));
    Ok(())
}

fn print_review(
    configuration: &Configuration,
    context: &OrgContext,
    review: &Review,
    users: &[UserSummary],
) {
    let state = if review.complete {
        i18n::remote_review_state_complete()
    } else {
        i18n::remote_review_state_open()
    };
    i18n::remote_review_line(
        &review.id.to_string(),
        &store_name(configuration, review.store),
        &day(review.due),
        &state,
    );
    for item in &review.items {
        i18n::remote_review_item(
            &context.describe(&item.principal, users),
            &format!("{:?}", item.role).to_ascii_lowercase(),
            &decision_text(item.decision),
        );
    }
}

fn reviews(configuration: &Configuration, org: Option<&str>) -> Result<()> {
    let context = OrgContext::open(configuration, org)?;
    let reviews = context.remote.client.reviews(context.id()).localized()?;
    if reviews.is_empty() {
        i18n::remote_no_reviews();
        return Ok(());
    }
    let users = context.remote.client.users(context.id()).localized()?;
    for review in &reviews {
        print_review(configuration, &context, review, &users);
    }
    Ok(())
}

fn decide_review(configuration: &Configuration, args: &OrgReviewDecideArgs) -> Result<()> {
    let context = OrgContext::open(configuration, args.org_selection.org.as_deref())?;
    let id = parse_id(&args.review)?;
    let review = context
        .remote
        .client
        .reviews(context.id())
        .localized()?
        .into_iter()
        .find(|review| review.id == id)
        .ok_or_else(|| anyhow::anyhow!(i18n::error_remote_unknown_review(&args.review)))?;
    let principal = context.principal(&args.principal)?;
    if !review.items.iter().any(|item| item.principal == principal) {
        anyhow::bail!(i18n::error_remote_not_under_review(
            &args.principal,
            &args.review
        ));
    }
    let decision = match args.decision {
        ReviewDecisionArg::Keep => Decision::Keep,
        ReviewDecisionArg::Revoke => Decision::Revoke,
    };
    context
        .remote
        .client
        .decide(context.id(), id, principal, decision)
        .localized()?;
    i18n::remote_review_decided(
        &decision_text(Some(decision)),
        &args.principal,
        &args.review,
    );
    if decision == Decision::Revoke {
        i18n::remote_review_revokes_pending();
    }
    Ok(())
}

fn scim_token(configuration: &Configuration, command: &OrgScimTokenCommands) -> Result<()> {
    match command {
        OrgScimTokenCommands::Create(args) => {
            let context = OrgContext::open(configuration, args.org.as_deref())?;
            let created = context
                .remote
                .client
                .create_scim_token(context.id())
                .localized()?;
            i18n::remote_scim_token_created(&created.id.to_string());
            println!("{}", created.token);
        }
        OrgScimTokenCommands::List(args) => {
            let context = OrgContext::open(configuration, args.org.as_deref())?;
            let tokens = context
                .remote
                .client
                .scim_tokens(context.id())
                .localized()?;
            if tokens.is_empty() {
                i18n::remote_no_scim_tokens();
            }
            for token in tokens {
                i18n::remote_scim_token_line(
                    &token.id.to_string(),
                    &crate::commands::audit::rfc3339(token.created),
                );
            }
        }
        OrgScimTokenCommands::Revoke(args) => {
            let context = OrgContext::open(configuration, args.org_selection.org.as_deref())?;
            let id = parse_id(&args.token)?;
            context
                .remote
                .client
                .revoke_scim_token(context.id(), id)
                .localized()?;
            i18n::remote_scim_token_revoked(&args.token);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::parser::utc_date;
    use crate::models::configuration::{RemoteLocation, StoreRegistration};

    #[test]
    fn due_days_print_the_day_the_user_named() {
        for date in ["1970-01-01", "2024-02-29", "2099-12-31"] {
            assert_eq!(day(utc_date(date).unwrap()), date);
        }
    }

    #[test]
    fn stores_print_under_their_local_name_or_identifier() {
        let registered = Id::random();
        let unregistered = Id::random();
        let configuration = Configuration {
            stores: vec![
                StoreRegistration {
                    name: "files".into(),
                    ..StoreRegistration::default()
                },
                StoreRegistration {
                    name: "team".into(),
                    remote: Some(RemoteLocation {
                        server: "https://pasejo.example".into(),
                        org: Id::random().to_string(),
                        store: registered.to_string(),
                    }),
                    ..StoreRegistration::default()
                },
            ],
            ..Configuration::default()
        };
        assert_eq!(store_name(&configuration, registered), "team");
        assert_eq!(
            store_name(&configuration, unregistered),
            unregistered.to_string()
        );
    }

    #[test]
    fn machines_are_named_with_a_prefix_and_people_by_email() {
        assert!(user_matches(
            "alice@example.com",
            "Alice@Example.com",
            "Alice",
            false
        ));
        assert!(!user_matches(
            "alice@example.com",
            "bob@example.com",
            "Alice",
            false
        ));
        assert!(user_matches("machine:ci", "", "ci", true));
        assert!(
            !user_matches("machine:ci", "", "ci", false),
            "a person named ci"
        );
        assert!(!user_matches("machine:ci", "", "deploy", true));
        assert!(
            !user_matches("", "", "ci", true),
            "an empty name never matches"
        );
        let machine = UserSummary {
            id: Id::random(),
            name: "ci".into(),
            email: String::new(),
            machine: true,
            active: true,
        };
        assert_eq!(user_display(&machine), "machine:ci");
        let person = UserSummary {
            name: "Alice".into(),
            email: "alice@example.com".into(),
            machine: false,
            ..machine
        };
        assert_eq!(user_display(&person), "alice@example.com");
    }

    #[test]
    fn decisions_print_in_the_user_language() {
        crate::cli::i18n::init_for_tests();
        assert_eq!(decision_text(Some(Decision::Keep)), "keep");
        assert_eq!(decision_text(Some(Decision::Revoke)), "revoke");
        assert_eq!(decision_text(None), "pending");
    }
}
