extern crate shared;

use shared::types;
use std::process::Command;
use std::rc::Rc;
use std::str;

#[no_mangle]
pub fn get_meta() -> types::Meta {
    let mut meta = types::Meta::new();
    meta.commandrc("drop", Rc::new(wrap(drop)));
    meta.commandrc("load", Rc::new(wrap(load)));
    meta.commandrc("reload", Rc::new(wrap(reload)));
    meta.commandrc("recompile", Rc::new(wrap(recompile)));
    meta
}

fn wrap(f: impl Fn(&mut types::Context, &str)) -> impl Fn(&mut types::Context, &str) {
    move |ctx: &mut types::Context, args| {
        if ctx.has_perm(types::PERM_ADMIN) {
            f(ctx, args)
        } else {
            ctx.reply("permission denied")
        }
    }
}

fn exec(
    ctx: &mut types::Context,
    args: &str,
    what: fn(&mut types::Context, &str) -> Result<(), String>,
) {
    for m in args.split(' ') {
        if m == "core" {
            ctx.reply("skipping core");
            continue;
        }
        match what(ctx, m) {
            Ok(()) => (),
            Err(e) => ctx.reply(&format!("{} failed: {}", m, e)),
        }
    }
    ctx.reply("done");
}

fn drop(ctx: &mut types::Context, args: &str) {
    exec(ctx, args, |ctx, m| ctx.bot().drop_module(m))
}

fn load(ctx: &mut types::Context, args: &str) {
    exec(ctx, args, |ctx, m| ctx.bot().load_module(m))
}

fn reload(ctx: &mut types::Context, args: &str) {
    exec(ctx, args, |ctx, m| {
        ctx.bot().drop_module(m)?;
        ctx.bot().load_module(m)
    })
}

fn recompile(ctx: &mut types::Context, args: &str) {
    let mut cmd = Command::new("cargo");
    cmd.arg("build");
    if !cfg!(debug_assertions) {
        cmd.arg("--release");
    }

    match cmd.output() {
        Ok(result) => {
            if result.status.success() {
                reload(ctx, args);
            } else {
                ctx.reply("compile failed:");
                for line in str::from_utf8(&result.stderr).unwrap().split('\n') {
                    if line.starts_with("   Compiling") {
                        continue;
                    }
                    if line == "" {
                        break;
                    }
                    ctx.reply(line);
                }
            }
        }
        Err(e) => ctx.reply(&format!("failed to run build: {}", e)),
    }
}
