// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

//! Access reviews.

use pasejo_protocol::api::{Decision, Review, ReviewDecision, ReviewRequest, Reviews};
use pasejo_protocol::log::Principal;
use pasejo_protocol::{Id, Millis};

use crate::error::Result;
use crate::transport::Client;

impl Client {
    /// Opens a review of `store`'s grants.
    ///
    /// # Errors
    ///
    /// Fails for users who administer neither the organization nor the
    /// store.
    pub fn open_review(
        &self,
        org: Id,
        store: Id,
        reviewers: Vec<Id>,
        due: Millis,
    ) -> Result<Review> {
        self.call(
            "POST",
            &format!("/v1/orgs/{org}/reviews"),
            &ReviewRequest {
                store,
                reviewers,
                due,
            },
        )
    }

    /// Reviews visible to the signed-in user.
    ///
    /// # Errors
    ///
    /// Fails when the server refuses.
    pub fn reviews(&self, org: Id) -> Result<Vec<Review>> {
        Ok(self
            .get::<Reviews>(&format!("/v1/orgs/{org}/reviews"))?
            .reviews)
    }

    /// Records a decision on one grant.
    ///
    /// # Errors
    ///
    /// Fails for non-reviewers and for decisions on one's own access.
    pub fn decide(
        &self,
        org: Id,
        review: Id,
        principal: Principal,
        decision: Decision,
    ) -> Result<Review> {
        self.call(
            "POST",
            &format!("/v1/orgs/{org}/reviews/{review}/decisions"),
            &ReviewDecision {
                principal,
                decision,
            },
        )
    }
}
