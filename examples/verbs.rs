//! Prove the four control verbs against a live anchord. No device needed.
//!
//! `src/anchor.rs`'s unit tests parse replies that were captured by hand.
//! This sends the real bytes to a real daemon and reads what comes back,
//! which is the only way to find out that a field is spelled `command`
//! and not `argv` — the class of bug a green suite is blind to, and the
//! one that has cost this project the most time
//! (`docs/09-testing.md#what-a-green-suite-does-not-prove`).
//!
//! ```sh
//! cargo run --example verbs
//! ```
//!
//! It cleans up after itself: everything it spawns, it purges.

use std::collections::BTreeMap;

use tally::anchor::{Client, Error, Spawn};
use tally::model;
use tally::slot::{Pages, Slot};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new(Client::default_path());
    println!("anchord at {}", client.path().display());

    let pages = Pages::default();
    let at = Slot {
        page: 8,
        row: 8,
        col: 8,
    };
    let address = at.label(&pages).ok_or("page 8 has no name")?;
    println!("using {address}, which is the far corner of the last page");

    let mut labels = BTreeMap::new();
    let _slot = labels.insert(tally::anchor::LABEL_SLOT.to_owned(), address.clone());
    let _spec = labels.insert(tally::anchor::LABEL_SPEC.to_owned(), "verbs".to_owned());
    let env = BTreeMap::new();
    let args = ["-c".to_owned(), "echo VERBS_OK; sleep 120".to_owned()];

    println!("\n-- create");
    let session = client.create(&Spawn {
        command: "/bin/sh",
        args: &args,
        cwd: None,
        env: &env,
        login_shell: false,
        labels: &labels,
    })?;
    println!("   {session}");

    println!("\n-- list: the labels came back");
    let sessions = client.list()?;
    let row = sessions
        .iter()
        .find(|s| s.id == session)
        .ok_or("the session we just made is not in the list")?;
    println!("   state={} labels={:?}", row.state, row.labels);
    assert_eq!(row.slot_label(), Some(address.as_str()), "tl.slot survived");
    assert!(!row.muted(), "nothing muted it");

    println!("\n-- label: mute, then un-mute");
    let mut set = BTreeMap::new();
    let _m = set.insert(tally::anchor::LABEL_MUTE.to_owned(), "1".to_owned());
    client.label(&session, &set, &[])?;
    let muted = client.list()?.into_iter().find(|s| s.id == session);
    println!(
        "   muted={}",
        muted.as_ref().is_some_and(tally::anchor::Session::muted)
    );
    assert!(muted.is_some_and(|s| s.muted()), "tl.mute did not stick");

    client.label(&session, &BTreeMap::new(), &[tally::anchor::LABEL_MUTE])?;
    let cleared = client.list()?.into_iter().find(|s| s.id == session);
    assert!(!cleared.is_some_and(|s| s.muted()), "tl.mute did not clear");
    println!("   cleared");

    println!("\n-- purge while running: anchord must refuse");
    match client.purge(&session) {
        Err(Error::Refused { code, message }) => println!("   refused: {code}: {message}"),
        Ok(()) => return Err("anchord purged a running session".into()),
        Err(e) => return Err(format!("wrong error: {e}").into()),
    }

    println!("\n-- terminate: synchronous to terminal state");
    client.terminate(&session)?;
    let after = client
        .list()?
        .into_iter()
        .find(|s| s.id == session)
        .ok_or("the row vanished")?;
    println!("   state={} exit={:?}", after.state, after.exit_code);

    println!("\n-- purgeable now sees it");
    let rows = model::purgeable(&client.list()?, &pages);
    println!("   {:?}", rows.get(&at));
    assert!(
        rows.get(&at).is_some_and(|ids| ids.contains(&session)),
        "the dead row is not purgeable"
    );

    println!("\n-- purge");
    client.purge(&session)?;
    assert!(
        !client.list()?.iter().any(|s| s.id == session),
        "it is still listed"
    );
    println!("   gone");

    println!("\n-- purge again: a typo must be reported, not silently fine");
    match client.purge(&session) {
        Err(Error::Refused { code, .. }) if code == "not_found" => println!("   not_found"),
        other => return Err(format!("expected not_found, got {other:?}").into()),
    }

    println!("\nall four verbs behave as docs/01-scope.md says");
    Ok(())
}
