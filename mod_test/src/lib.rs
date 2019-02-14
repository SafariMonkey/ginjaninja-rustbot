extern crate shared;

use shared::types;
use std::collections::HashMap;

#[no_mangle]
pub fn get_meta() -> types::Meta {
    let mut commands: HashMap<String, types::Command> = HashMap::new();
    commands.insert("test".to_string(), |ctx, args| {
        println!("test command with args = {:?}", args);
        ctx.reply("beep boop");
    });
    types::Meta { commands }
}
