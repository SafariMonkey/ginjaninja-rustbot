use crate::build_message;
use crate::utils::*;
use rustbot::prelude::*;
use std::collections::BTreeMap;

pub(crate) fn status(ctx: &dyn Context, args: &str) -> Result<()> {
    let server = resolve_server(ctx, args)?;
    let resp = get_topic_map(server.address, b"status=2")?;

    ctx.reply(Message::Simple(render_fields(
        &resp,
        &[
            ("Players", "players"),
            ("Active Players", "active_players"),
            ("Mode", "mode"),
            ("Station Time", "stationtime"),
            ("Round Duration", "roundduration"),
            ("Map", "map"),
        ],
    )))
}

pub(crate) fn address(ctx: &dyn Context, args: &str) -> Result<()> {
    let server = resolve_server(ctx, args)?;

    ctx.reply(Message::Simple(format!("byond://{}", server.address)))
}

pub(crate) fn revision(ctx: &dyn Context, args: &str) -> Result<()> {
    let server = resolve_server(ctx, args)?;
    let resp = get_topic_map(server.address, b"revision")?;

    ctx.reply(Message::Simple(build_message!(
        resp,
        "Revision: {} on {} at {}. Game ID: {}. DM: {}.{}; DD: {}.{}",
        revision,
        branch,
        date,
        gameid,
        dm_version,
        dm_build,
        dd_version,
        dd_build
    )))
}

pub(crate) fn mode(ctx: &dyn Context, args: &str) -> Result<()> {
    let server = resolve_server(ctx, args)?;
    let resp = get_topic_map(server.address, b"status=2")?;

    ctx.reply(Message::Simple(build_message!(resp, "Mode: {}", mode)))
}

pub(crate) fn admins(ctx: &dyn Context, args: &str) -> Result<()> {
    let server = resolve_server(ctx, args)?;
    let resp = get_topic_map(server.address, b"status=2")?;

    let admins = parse_urlencoded(
        resp.get("adminlist")
            .ok_or_else(|| Error::msg("got status=2 Topic response without adminlist key"))?,
    );

    if admins.is_empty() {
        ctx.reply(Message::Simple("No admins online.".to_string()))
    } else {
        ctx.reply(Message::List {
            prefix: format!("Admins ({}): ", admins.len()).into(),
            sep: "; ".into(),
            items: admins
                .iter()
                .map(|(name, rank)| format!("{} is {} {}", name, a(rank), rank).into())
                .collect::<Vec<_>>(),
        })
    }
}

fn a(s: &str) -> &'static str {
    if s.starts_with(|c| "aeiouAEIOU".contains(c)) {
        "an"
    } else {
        "a"
    }
}

pub(crate) fn players(ctx: &dyn Context, args: &str) -> Result<()> {
    let server = resolve_server(ctx, args)?;
    let resp = get_topic_map(server.address, b"status=2")?;

    let players = parse_urlencoded(
        resp.get("playerlist")
            .ok_or_else(|| Error::msg("got status=2 Topic response without playerlist key"))?,
    );

    if players.is_empty() {
        ctx.reply(Message::Simple("No players online.".to_string()))
    } else {
        ctx.reply(Message::List {
            prefix: format!("Players ({}): ", players.len()).into(),
            sep: ", ".into(),
            items: players.keys().map(Into::into).collect(),
        })
    }
}

pub(crate) fn manifest(ctx: &dyn Context, args: &str) -> Result<()> {
    let server = resolve_server(ctx, args)?;
    let resp = get_topic_map(server.address, b"manifest")?;

    let resp = resp
        .iter()
        .map(|(k, v)| (k, parse_urlencoded(v)))
        .collect::<BTreeMap<_, _>>();

    if resp.is_empty() {
        ctx.reply(Message::Simple("Manifest is empty.".to_string()))
    } else {
        let mut lines = vec![];
        for (dept, list) in resp {
            lines.push(format!(
                "{}: {}",
                dept,
                list.iter()
                    .map(|(name, job)| format!("{}: {}", name, job))
                    .collect::<Vec<_>>()
                    .join("; ")
            ));
        }
        ctx.reply(Message::Simple(lines.join("\n")))
    }
}
