//! Slot addresses: `page:row:col`, the string a human types into a shell.
//!
//! ```text
//! tl.slot = "work:3:5"
//!            │    │ └── col 1-8, 1 at the LEFT
//!            │    └──── row 1-8, 1 at the TOP
//!            └───────── page name, declared in slots.toml
//! ```
//!
//! Row 1 is the top because a label is a human artefact and reads top-down
//! like everything else on a screen. The hardware numbers from the bottom;
//! that flip lives in [`crate::surface::addr`] and nowhere else.

use crate::surface::Button;

/// How many pages the top row can select.
pub const PAGES: u8 = 8;

/// A resolved address: a page *index*, and a position on it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Slot {
    /// 1-8, left to right along the top row.
    pub page: u8,
    /// 1-8, 1 is the top row.
    pub row: u8,
    /// 1-8, 1 is the left column.
    pub col: u8,
}

impl Slot {
    /// The button this slot occupies when its page is on the grid.
    pub fn button(self) -> Button {
        Button::Grid {
            row: self.row,
            col: self.col,
        }
    }
}

/// The ordered page names, which is what maps a name onto a top-row button.
///
/// Ordering lives in config rather than in the label because *which button
/// a page sits under* is a layout preference, not an identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pages {
    names: Vec<String>,
}

impl Default for Pages {
    /// Eight pages named `"1"` through `"8"`.
    ///
    /// This is what you get with no `slots.toml`, which is deliberate:
    /// M1 has no spawning, so it needs no specs, and the grid still lights
    /// up for anything a shell script labelled.
    fn default() -> Self {
        Self {
            names: (1..=PAGES).map(|n| n.to_string()).collect(),
        }
    }
}

impl Pages {
    /// Declare pages in top-row order. Anything past the eighth is ignored,
    /// because there is no ninth button to put it under.
    pub fn new(names: impl IntoIterator<Item = String>) -> Self {
        Self {
            names: names.into_iter().take(PAGES as usize).collect(),
        }
    }

    /// The 1-based top-row position of a page, or `None` if undeclared.
    pub fn index_of(&self, name: &str) -> Option<u8> {
        let at = self.names.iter().position(|n| n == name)?;
        u8::try_from(at + 1).ok()
    }

    /// The name at a 1-based index.
    pub fn name_of(&self, index: u8) -> Option<&str> {
        self.names
            .get(usize::from(index).checked_sub(1)?)
            .map(String::as_str)
    }

    /// How many pages are declared.
    pub fn len(&self) -> usize {
        self.names.len()
    }

    /// Whether no pages are declared at all.
    pub fn is_empty(&self) -> bool {
        self.names.is_empty()
    }
}

/// Parse a `tl.slot` value.
///
/// Returns `None` for anything that is not `name:1-8:1-8` naming a declared
/// page. **Ignored, not clamped and not repaired**: the value was written by
/// something that disagrees with us about the address space, and putting it
/// somewhere arbitrary is worse than leaving it off the grid.
pub fn parse(label: &str, pages: &Pages) -> Option<Slot> {
    let mut parts = label.split(':');
    let page = pages.index_of(parts.next()?)?;
    let row = coordinate(parts.next()?)?;
    let col = coordinate(parts.next()?)?;
    if parts.next().is_some() {
        return None; // "a:1:2:3" is not an address with a suffix, it is a typo
    }
    Some(Slot { page, row, col })
}

fn coordinate(text: &str) -> Option<u8> {
    let n: u8 = text.parse().ok()?;
    (1..=8).contains(&n).then_some(n)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn named() -> Pages {
        Pages::new(["work".to_owned(), "infra".to_owned()])
    }

    #[test]
    fn a_well_formed_address_resolves() {
        assert_eq!(
            parse("work:3:5", &named()),
            Some(Slot {
                page: 1,
                row: 3,
                col: 5
            })
        );
        assert_eq!(
            parse("infra:8:8", &named()),
            Some(Slot {
                page: 2,
                row: 8,
                col: 8
            })
        );
    }

    /// Row 1 is the top in a label and the bottom on the wire. Confirmed
    /// against hardware in M0.
    #[test]
    fn row_one_is_the_top_and_becomes_the_bottom_on_the_wire() {
        let top = parse("work:1:1", &named()).expect("top-left");
        assert_eq!(top.button().index(), 81);
        let bottom = parse("work:8:1", &named()).expect("bottom-left");
        assert_eq!(bottom.button().index(), 0x0B);
    }

    /// Ignored, not clamped. Each of these was written by something that
    /// disagrees with us about the address space.
    #[test]
    fn malformed_addresses_are_ignored_rather_than_repaired() {
        let pages = named();
        for bad in [
            "nope:1:1",   // undeclared page
            "work:0:1",   // row below the grid
            "work:9:1",   // row above it
            "work:1:0",   // column left of it
            "work:1:9",   // column right of it
            "work:1",     // missing a coordinate
            "work:1:2:3", // one too many
            "work:a:1",   // not a number
            "work:-1:1",  // negative
            "work: 1:1",  // whitespace
            "work:01:1",  // leading zero is fine for parse, kept for the record
            "",           // empty
            ":::",        // punctuation only
            "work:1:1 ",  // trailing space
        ] {
            let got = parse(bad, &pages);
            if bad == "work:01:1" {
                assert_eq!(
                    got,
                    Some(Slot {
                        page: 1,
                        row: 1,
                        col: 1
                    }),
                    "{bad:?}"
                );
            } else {
                assert_eq!(got, None, "{bad:?} should be ignored");
            }
        }
    }

    #[test]
    fn with_no_config_pages_are_numbered() {
        let pages = Pages::default();
        assert_eq!(pages.len(), 8);
        assert_eq!(
            parse("1:4:4", &pages),
            Some(Slot {
                page: 1,
                row: 4,
                col: 4
            })
        );
        assert_eq!(
            parse("8:1:1", &pages),
            Some(Slot {
                page: 8,
                row: 1,
                col: 1
            })
        );
        // There is no ninth button to put a ninth page under.
        assert_eq!(parse("9:1:1", &pages), None);
    }

    #[test]
    fn pages_past_the_eighth_are_dropped_not_wrapped() {
        let many = Pages::new((1..=12).map(|n| format!("p{n}")));
        assert_eq!(many.len(), 8);
        assert_eq!(many.index_of("p8"), Some(8));
        assert_eq!(many.index_of("p9"), None);
    }

    #[test]
    fn names_and_indices_round_trip() {
        let pages = named();
        for index in 1..=2 {
            let name = pages.name_of(index).expect("declared");
            assert_eq!(pages.index_of(name), Some(index));
        }
        assert_eq!(pages.name_of(0), None);
        assert_eq!(pages.name_of(3), None);
    }
}
