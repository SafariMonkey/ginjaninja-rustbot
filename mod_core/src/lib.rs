extern crate shared;

use shared::prelude::*;
use std::process::Command as ProcessCommand;
use std::str;

#[no_mangle]
pub fn get_meta() -> Meta {
    let mut meta = Meta::new();
    meta.cmd("drop", Command::new(drop));
    meta.cmd("load", Command::new(load));
    meta.cmd("reload", Command::new(reload));
    meta.cmd("recompile", Command::new(recompile));
    meta
}

fn exec(ctx: &mut Context, args: &str, what: fn(&mut Context, &str) -> Result<()>) -> Result<()> {
    for m in args.split(' ') {
        if m == "core" {
            ctx.reply("skipping core")?;
            continue;
        }
        match what(ctx, m) {
            Ok(()) => Ok(()),
            Err(e) => ctx.reply(&format!("{} failed: {}", m, e)),
        }?;
    }
    ctx.reply("done")
}

fn drop(ctx: &mut Context, args: &str) -> Result<()> {
    exec(ctx, args, |ctx, m| ctx.bot.drop_module(m))
}

fn load(ctx: &mut Context, args: &str) -> Result<()> {
    exec(ctx, args, |ctx, m| ctx.bot.load_module(m))
}

fn reload(ctx: &mut Context, args: &str) -> Result<()> {
    exec(ctx, args, |ctx, m| {
        ctx.bot.drop_module(m)?;
        ctx.bot.load_module(m)
    })
}

fn recompile(ctx: &mut Context, args: &str) -> Result<()> {
    let mut cmd = ProcessCommand::new("cargo");
    cmd.arg("build");
    if !cfg!(debug_assertions) {
        cmd.arg("--release");
    }

    match cmd.output() {
        Ok(result) => {
            if result.status.success() {
                reload(ctx, args)
            } else {
                ctx.reply("compile failed:")?;
                for line in str::from_utf8(&result.stderr).unwrap().split('\n') {
                    if line.starts_with("   Compiling") {
                        continue;
                    }
                    if line == "" {
                        break;
                    }
                    ctx.reply(line)?;
                }
                Ok(())
            }
        }
        Err(e) => ctx.reply(&format!("failed to run build: {}", e)),
    }
}
