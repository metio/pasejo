// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use std::collections::BTreeSet;

/// Labels shorter than this differ from too many real domains by a single
/// character for one edit to mean anything.
const MIN_EDIT_LABEL_LEN: usize = 4;

/// Returns the registrable domains among `stored_hosts` that differ from the
/// registrable domain of `query_host` but could be mistaken for it, sorted
/// and without duplicates. Hosts are expected in the canonical form the
/// matcher compares: IDNA-encoded, lowercase, IPv6 unbracketed.
///
/// Two domains are look-alikes when their labels (the registrable domain
/// without its public suffix) have the same confusable skeleton under any
/// public suffixes, or when they share a public suffix and their skeletons
/// are one inserted, removed, replaced or swapped character apart. When the
/// queried registrable domain is itself among the stored ones, the user
/// already knows the site and nothing is reported.
pub fn lookalikes<I, S>(query_host: &str, stored_hosts: I) -> Vec<String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let Some(query) = Site::parse(query_host) else {
        return vec![];
    };
    let stored: BTreeSet<Site> = stored_hosts
        .into_iter()
        .filter_map(|host| Site::parse(host.as_ref()))
        .collect();
    if stored
        .iter()
        .any(|site| site.registrable == query.registrable)
    {
        return vec![];
    }
    stored
        .into_iter()
        .filter(|site| query.resembles(site))
        .map(|site| site.registrable)
        .collect()
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct Site {
    registrable: String,
    suffix: String,
    skeleton: Vec<char>,
}

impl Site {
    fn parse(host: &str) -> Option<Self> {
        if host.parse::<std::net::IpAddr>().is_ok() {
            return None;
        }
        let registrable = psl::domain_str(host)?.to_lowercase();
        let suffix = psl::suffix_str(&registrable)?.to_owned();
        let label = registrable
            .strip_suffix(suffix.as_str())?
            .strip_suffix('.')
            .filter(|label| !label.is_empty())?;
        let skeleton = skeleton(&url::quirks::domain_to_unicode(label));
        Some(Self {
            registrable,
            suffix,
            skeleton,
        })
    }

    fn resembles(&self, other: &Self) -> bool {
        if self.registrable == other.registrable {
            return false;
        }
        if self.skeleton == other.skeleton {
            return true;
        }
        self.suffix == other.suffix
            && self.skeleton.len().min(other.skeleton.len()) >= MIN_EDIT_LABEL_LEN
            && within_one_edit(&self.skeleton, &other.skeleton)
    }
}

/// The Unicode TR39 confusable skeleton of `label`, lowercased, with the
/// ASCII confusions TR39 leaves apart folded together: `i` and `l`, `vv`
/// and `w`. TR39 itself maps `0` to `O`, `1` and `I` to `l`, and `m` to
/// `rn`.
fn skeleton(label: &str) -> Vec<char> {
    let lowered: String = unicode_security::skeleton(label)
        .flat_map(char::to_lowercase)
        .collect();
    lowered
        .replace("vv", "w")
        .replace("rn", "m")
        .chars()
        .map(|c| if c == 'i' { 'l' } else { c })
        .collect()
}

/// Optimal-string-alignment distance of exactly one between two different
/// sequences: one insertion, deletion, substitution or adjacent swap.
fn within_one_edit(a: &[char], b: &[char]) -> bool {
    let (short, long) = if a.len() <= b.len() { (a, b) } else { (b, a) };
    let prefix = short.iter().zip(long).take_while(|(x, y)| x == y).count();
    match long.len() - short.len() {
        0 => {
            prefix < short.len()
                && (short[prefix + 1..] == long[prefix + 1..]
                    || (prefix + 1 < short.len()
                        && short[prefix] == long[prefix + 1]
                        && short[prefix + 1] == long[prefix]
                        && short[prefix + 2..] == long[prefix + 2..]))
        }
        1 => short[prefix..] == long[prefix + 1..],
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The IDNA (punycode) form of a Unicode host, as the matcher sees it.
    fn ascii(host: &str) -> String {
        url::Host::parse(host).unwrap().to_string()
    }

    fn lookalikes_of(query: &str, stored: &[&str]) -> Vec<String> {
        lookalikes(query, stored)
    }

    #[test]
    fn digit_zero_reads_as_letter_o() {
        assert_eq!(lookalikes_of("g00gle.com", &["google.com"]), ["google.com"]);
    }

    #[test]
    fn digit_one_and_letter_i_read_as_letter_l() {
        assert_eq!(lookalikes_of("paypa1.com", &["paypal.com"]), ["paypal.com"]);
        assert_eq!(lookalikes_of("paypai.com", &["paypal.com"]), ["paypal.com"]);
    }

    #[test]
    fn letters_r_and_n_read_as_letter_m() {
        assert_eq!(
            lookalikes_of("rnicrosoft.com", &["microsoft.com"]),
            ["microsoft.com"]
        );
    }

    #[test]
    fn two_letters_v_read_as_letter_w() {
        assert_eq!(
            lookalikes_of("vvikipedia.org", &["wikipedia.org"]),
            ["wikipedia.org"]
        );
    }

    #[test]
    fn cyrillic_homoglyph_in_a_punycode_query_is_detected() {
        let query = ascii("\u{0430}pple.com");
        assert!(query.starts_with("xn--"));
        assert_eq!(lookalikes_of(&query, &["apple.com"]), ["apple.com"]);
    }

    #[test]
    fn greek_homoglyph_in_a_punycode_query_is_detected() {
        let query = ascii("g\u{03bf}ogle.com");
        assert_eq!(lookalikes_of(&query, &["google.com"]), ["google.com"]);
    }

    #[test]
    fn homoglyph_in_the_top_level_domain_is_detected() {
        let query = ascii("example.c\u{043e}m");
        assert_eq!(lookalikes_of(&query, &["example.com"]), ["example.com"]);
    }

    #[test]
    fn same_label_under_another_public_suffix_is_detected() {
        assert_eq!(
            lookalikes_of("example.net", &["example.com"]),
            ["example.com"]
        );
        assert_eq!(
            lookalikes_of("example.co.uk", &["example.com"]),
            ["example.com"]
        );
        assert_eq!(
            lookalikes_of("example.co", &["example.com"]),
            ["example.com"]
        );
    }

    #[test]
    fn one_edit_on_the_label_is_detected() {
        for query in [
            "exammple.com",
            "exmple.com",
            "exanple.com",
            "exmaple.com",
            "exam-ple.com",
        ] {
            assert_eq!(
                lookalikes_of(query, &["example.com"]),
                ["example.com"],
                "{query}"
            );
        }
    }

    #[test]
    fn two_edits_on_the_label_are_not_a_lookalike() {
        assert!(lookalikes_of("examqqe.com", &["example.com"]).is_empty());
        assert!(lookalikes_of("exampleab.com", &["example.com"]).is_empty());
    }

    #[test]
    fn one_edit_under_another_public_suffix_is_not_a_lookalike() {
        assert!(lookalikes_of("exanple.net", &["example.com"]).is_empty());
    }

    #[test]
    fn one_edit_on_a_short_label_is_not_a_lookalike() {
        assert!(lookalikes_of("abd.com", &["abc.com"]).is_empty());
        assert!(lookalikes_of("ab.com", &["abc.com"]).is_empty());
    }

    #[test]
    fn unrelated_domains_are_not_lookalikes() {
        assert!(lookalikes_of("gitlab.com", &["github.com"]).is_empty());
        assert!(lookalikes_of("example.org", &["unrelated.org"]).is_empty());
    }

    #[test]
    fn subdomains_compare_by_their_registrable_domain() {
        assert_eq!(
            lookalikes_of("login.examp1e.com", &["accounts.example.com"]),
            ["example.com"]
        );
    }

    #[test]
    fn a_query_whose_own_domain_is_stored_has_no_lookalikes() {
        assert!(lookalikes_of("www.examp1e.com", &["examp1e.com", "example.com"]).is_empty());
    }

    #[test]
    fn the_queried_domain_itself_is_never_reported() {
        assert!(lookalikes_of("www.example.com", &["example.com"]).is_empty());
    }

    #[test]
    fn results_are_sorted_and_deduplicated() {
        assert_eq!(
            lookalikes_of(
                "example.org",
                &[
                    "www.example.net",
                    "example.com",
                    "login.example.com",
                    "example.net",
                ]
            ),
            ["example.com", "example.net"]
        );
    }

    #[test]
    fn lookalikes_are_named_in_their_ascii_form() {
        let stored = ascii("m\u{00fc}ller.de");
        assert_eq!(lookalikes_of("muller.de", &[stored.as_str()]), [stored]);
    }

    fn one_edit(a: &str, b: &str) -> bool {
        let a: Vec<char> = a.chars().collect();
        let b: Vec<char> = b.chars().collect();
        within_one_edit(&a, &b) && within_one_edit(&b, &a)
    }

    #[test]
    fn within_one_edit_counts_single_edits_at_every_position() {
        assert!(one_edit("abcd", "xbcd"));
        assert!(one_edit("abcd", "abcx"));
        assert!(one_edit("abcd", "bacd"));
        assert!(one_edit("abcd", "abdc"));
        assert!(one_edit("abcd", "abcde"));
        assert!(one_edit("abcd", "xabcd"));
        assert!(one_edit("", "a"));
    }

    #[test]
    fn within_one_edit_rejects_equal_and_distant_sequences() {
        assert!(!one_edit("abcd", "abcd"));
        assert!(!one_edit("", ""));
        assert!(!one_edit("abcd", "badc"));
        assert!(!one_edit("abcd", "acbx"));
        assert!(!one_edit("abcd", "abcdef"));
        assert!(!one_edit("abcd", "xbcy"));
    }

    #[test]
    fn ip_addresses_have_no_lookalikes() {
        assert!(lookalikes_of("192.168.0.1", &["192.168.0.2"]).is_empty());
        assert!(lookalikes_of("example.com", &["10.0.0.1", "::1"]).is_empty());
    }

    #[test]
    fn hosts_without_a_registrable_domain_have_no_lookalikes() {
        assert!(lookalikes_of("co.uk", &["example.co.uk"]).is_empty());
        assert!(lookalikes_of("", &["example.com"]).is_empty());
        assert!(lookalikes_of("example.com", &["", "com"]).is_empty());
    }
}
