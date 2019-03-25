#[macro_use]
extern crate nom;
extern crate rand;
extern crate shared;

mod dice;

use shared::prelude::*;

#[no_mangle]
pub fn get_meta() -> Meta {
    let mut meta = Meta::new();
    meta.cmd("dice", Command::new(cmd_dice));
    meta
}

fn cmd_dice(ctx: &Context, args: &str) -> Result<()> {
    let v = dice::parse(args)?;
    let result = dice::eval(v)?;
    ctx.say(result.as_str())
}
