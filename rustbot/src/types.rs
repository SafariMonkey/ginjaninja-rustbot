#![allow(non_upper_case_globals)]

use parking_lot::Mutex;
use postgres::types::FromSql;
use postgres::Connection;
use std::sync::Arc;

use types::Message::*;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

bitflags! {
    pub struct Perms: u64 {
        const None     = 0x00000000;
        const Admin    = 0x00000001;
        const Raw      = 0x00000002;
        const Database = 0x00000004;
        const Eval     = 0x00000008;
        const Modules  = 0x00000010;
    }
}

impl std::fmt::Display for Perms {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::result::Result<(), std::fmt::Error> {
        write!(f, "{:?}", self)?;

        let diff = self.bits & !Perms::all().bits;
        if diff != 0 {
            write!(f, " | 0x{:x}", diff)?;
        }

        Ok(())
    }
}

impl FromSql for Perms {
    fn from_sql(
        ty: &postgres::types::Type,
        raw: &[u8],
    ) -> std::result::Result<Self, Box<dyn std::error::Error + 'static + Send + Sync>> {
        i64::from_sql(ty, raw).map(|i| Perms { bits: i as u64 })
    }
    fn accepts(ty: &postgres::types::Type) -> bool {
        i64::accepts(ty)
    }
}

pub type CommandFn = dyn Fn(&dyn Context, &str) -> Result<()> + Send + Sync;
#[derive(Clone)]
pub struct Command {
    pub function: Arc<CommandFn>,
    pub req_perms: Perms,
}

impl Command {
    pub fn new(f: fn(&dyn Context, &str) -> Result<()>) -> Self {
        Self::arc(Arc::new(f))
    }
    pub fn arc(f: Arc<CommandFn>) -> Self {
        return Self {
            function: f,
            req_perms: Perms::None,
        };
    }
    pub fn req_perms(&self, p: Perms) -> Self {
        let mut s = self.clone();
        s.req_perms.insert(p);
        s
    }
    pub fn call(&self, ctx: &dyn Context, args: &str) -> Result<()> {
        if !ctx.perms()?.contains(self.req_perms) {
            return Ok(());
        }

        (self.function)(ctx, args)
    }
}

pub type DeinitFn = dyn FnMut(&dyn Bot) -> Result<()> + Send + Sync;

pub trait Meta {
    fn cmd(&mut self, name: &str, cmd: Command);
    fn deinit(&mut self, f: Box<DeinitFn>);
}

pub trait Bot {
    fn load_module(&self, &str) -> Result<()>;
    fn drop_module(&self, &str) -> Result<()>;
    fn sql(&self) -> &Mutex<Connection>;

    fn irc_send_privmsg(&self, &str, &str, &str) -> Result<()>;
    fn irc_send_raw(&self, &str, &str) -> Result<()>;

    fn dis_send_message(&self, &str, &str, &str, &str, bool) -> Result<()>;
}

pub trait Context {
    fn bot(&self) -> &(dyn Bot + Sync);
    fn say(&self, &str) -> Result<()>;
    fn reply(&self, Message) -> Result<()>;
    fn perms(&self) -> Result<Perms>;
    fn source_str(&self) -> String;
}

pub enum Message {
    Simple(String),
    Code(String),
}

fn paste_max_lines(input: String, max_lines: usize) -> Result<(Vec<String>, Option<String>)> {
    let lines: Vec<String> = input.split("\n").map(|l| l.to_string()).collect();
    if lines.len() > max_lines {
        let client = reqwest::Client::new();
        let mut result = client.post("http://ix.io").form(&[("f:1", input)]).send()?;

        let url = result.text()?;

        Ok((
            lines[0..max_lines - 1].to_vec(),
            Some(format!("[full message: {}]", url.trim())),
        ))
    } else {
        Ok((lines, None))
    }
}

impl Message {
    pub fn format_irc(self) -> Result<Vec<String>> {
        match self {
            Simple(s) | Code(s) => match paste_max_lines(s, 3)? {
                (lines, None) => Ok(lines),
                (mut lines, Some(extra)) => {
                    lines.push(extra);
                    Ok(lines)
                }
            },
        }
    }
    pub fn format_discord(self) -> Result<String> {
        match self {
            Simple(s) => match paste_max_lines(s, 11)? {
                (lines, None) => Ok(lines.join("\n")),
                (lines, Some(extra)) => Ok(format!("{}\n{}", lines.join("\n"), extra)),
            },
            Code(s) => {
                if !s.contains('\n') {
                    Ok(format!("`{}`", s))
                } else {
                    match paste_max_lines(s, 11)? {
                        (lines, None) => Ok(format!("```\n{}\n```", lines.join("\n"))),
                        (lines, Some(extra)) => Ok(format!("```\n{}\n```{}", lines.join("\n"), extra)),
                    }
                }
            }
        }
    }
}
