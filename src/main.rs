use std::{collections::HashMap, time::Duration};

use dbus::channel::MatchingReceiver;
use dbus::Message;
use dbus::{arg, blocking::Connection};
use dbus::{arg::RefArg, message::MatchRule};

fn main() {
    let conn = Connection::new_session().expect("D-Bus connection failed");
    let mut rule = MatchRule::new();

    let proxy = conn.with_proxy(
        "org.mpris.MediaPlayer2.spotify",
        "/org/mpris/MediaPlayer2",
        Duration::from_millis(5000),
    );
    let result: Result<(), dbus::Error> = proxy.method_call(
        "org.freedesktop.DBus.Monitoring",
        "BecomeMonitor",
        (vec![rule.match_str()], 0u32),
    );
    let mut old_song = String::new();
    if result.is_ok() {
        conn.start_receive(
            rule,
            Box::new(move |msg, _| {
                handle_message(&msg, &mut old_song);
                true
            }),
        );
    } else {
        rule.eavesdrop = true;
        conn.add_match(rule, move |_: (), _, msg| {
            handle_message(&msg, &mut old_song);
            true
        })
        .expect("add_match failed");
    }
    loop {
        conn.process(Duration::from_millis(1000)).unwrap();
    }
}

// hacky ugly handling
fn handle_message(msg: &Message, old_song: &mut String) {
    let song = get_song(&msg);
    if song.is_none() {
        return;
    }
    if let Some(song) = song {
        if &song == old_song {
            return;
        }
        *old_song = String::from(song);
        notify_rust::Notification::new()
            .summary("Now Playing!")
            .body(&old_song)
            .icon("spotify")
            .show()
            .unwrap();
    }
}

fn get_artist_title_map<'a>(
    iter: &mut Box<dyn Iterator<Item = &'a dyn RefArg> + 'a>,
) -> HashMap<&'a str, &'a dyn RefArg> {
    let mut map = HashMap::new();
    let arr: Vec<_> = iter.collect();
    for i in (0..arr.len()).step_by(2) {
        match arr[i].as_str() {
            Some(val) => {
                if val == "xesam:artist" {
                    map.insert("artist", arr[i + 1]);
                } else if val == "xesam:title" {
                    map.insert("title", arr[i + 1]);
                }
            }
            None => continue,
        }
    }
    map
}

fn get_song(msg: &Message) -> Option<String> {
    let item1: (Option<arg::PropMap>, Option<arg::PropMap>) = msg.get2();
    let item1 = item1.1;
    if item1.is_none() {
        return None;
    }
    let item1 = item1.unwrap();
    let metadata = item1.get("Metadata").unwrap();
    let metadata = &metadata.0;
    let iter = metadata.as_iter();
    let map = get_artist_title_map(&mut iter.unwrap());
    let artist = format!(
        "{:?}",
        map.get("artist")
            .unwrap()
            .as_iter()
            .unwrap()
            .next()
            .unwrap()
    )
    .replace('[', "")
    .replace(']', "")
    .replace('"', "");
    let title = format!(
        "{:?}",
        map.get("title").unwrap().as_iter().unwrap().next().unwrap()
    )
    .replace('[', "")
    .replace(']', "")
    .replace('"', "");

    Some(format!("{} - {}", artist, title))
}
