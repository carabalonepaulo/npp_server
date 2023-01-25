use async_std::{
    channel::{unbounded, Receiver, Sender},
    fs, task,
};
use futures::StreamExt;
use rlua::{FromLuaMulti, Function, Lua, Table, ToLuaMulti};

use crate::{
    events::{ListenerEvent, LuaEvent},
    generic_result::GenericResult,
    module::server,
};

const GLOBAL_MODULES: &str = "modules";

pub async fn run(
    sender: Sender<LuaEvent>,
    mut receiver: Receiver<ListenerEvent>,
) -> GenericResult<()> {
    println!("Lua started!");

    let state = Lua::new();
    add_global_modules(&state, sender.clone());

    loop {
        if let Some(ev) = receiver.next().await {
            match ev {
                ListenerEvent::ClientConnected(id) => {
                    call(&state, "server", "on_client_connected", (id))
                }
                ListenerEvent::CliendDisconnected(id) => {
                    call(&state, "server", "on_client_disconnected", (id))
                }
                ListenerEvent::LineReceived(id, line) => {
                    call(&state, "server", "on_line_received", (id, line));
                }
            }
        } else {
            break;
        }
    }

    println!("Lua finalized.");

    Ok(())
}

/*
rlua::function::Function
pub fn call<A, R>(&self, args: A) -> Result<R>
where
    A: ToLuaMulti<'lua>,
    R: FromLuaMulti<'lua>,

fn call<T>(&self, func_id: ValueId, args: T)
    where
        T: for<'a> rlua::ToLuaMulti<'a>,
    {
        self.state.context(move |ctx| {
            let globals = ctx.globals();
            let func: rlua::Function = globals.get(func_id as usize).unwrap();
            func.call::<_, ()>(args).unwrap();
        });
    }
*/

fn call<A>(state: &Lua, module: &str, function: &str, args: A)
where
    A: for<'a> ToLuaMulti<'a>,
{
    state.context(move |ctx| {
        let globals = ctx.globals();
        let modules: Table = globals.get(GLOBAL_MODULES).unwrap();
        let server: Table = modules.get(module).unwrap();
        let callback: Function = server.get(function).unwrap();
        callback.call::<_, ()>(args);
    });
}

fn add_global_modules(state: &Lua, sender: Sender<LuaEvent>) {
    state.context(|ctx| {
        ctx.load(
            r#"
local old_require = require
_G.require = function(path)
  if modules[path] then
    return modules[path]
  else
  end
  return old_require(path)
end
        "#,
        )
        .exec();

        ctx.globals()
            .set(GLOBAL_MODULES, ctx.create_table().unwrap())
            .unwrap();
    });

    server::register(state, sender);
}
