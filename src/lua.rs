use async_std::channel::{Receiver, Sender};
use futures::StreamExt;
use rlua::{Function, Lua, Table, ToLuaMulti};

use crate::{listener, module::server};

const GLOBAL_MODULES: &str = "modules";

pub enum Command {
    ClientConnected(usize),
    ClientDisconnected(usize),
    LineReceived(usize, String),
    Shutdown,
}

pub async fn run(sender: Sender<listener::Command>, mut receiver: Receiver<Command>) {
    let state = Lua::new();
    add_global_modules(&state, sender.clone());

    loop {
        if let Some(ev) = receiver.next().await {
            match ev {
                Command::ClientConnected(id) => call(&state, "server", "on_client_connected", id),
                Command::ClientDisconnected(id) => {
                    call(&state, "server", "on_client_disconnected", id)
                }
                Command::LineReceived(id, line) => {
                    call(&state, "server", "on_line_received", (id, line));
                }
                Command::Shutdown => break,
            }
        } else {
            break;
        }
    }

    call(&state, "server", "on_finalize", ());
}

fn call<A>(state: &Lua, module: &str, function: &str, args: A)
where
    A: for<'a> ToLuaMulti<'a>,
{
    state.context(move |ctx| {
        let globals = ctx.globals();
        let modules: Table = globals.get(GLOBAL_MODULES).unwrap();
        let server: Table = modules.get(module).unwrap();
        let callback: Function = server.get(function).unwrap();
        callback.call::<_, ()>(args).unwrap();
    });
}

fn add_global_modules(state: &Lua, listener_sender: Sender<listener::Command>) {
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
        .exec()
        .unwrap();

        ctx.globals()
            .set(GLOBAL_MODULES, ctx.create_table().unwrap())
            .unwrap();
    });

    server::register(state, listener_sender);
}
