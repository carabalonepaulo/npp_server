use std::fs;

use async_std::channel::Sender;
use rlua::{Function, Lua, Table};

use crate::listener::Command;

pub fn register(state: &Lua, listener_sender: Sender<Command>) {
    let code = fs::read_to_string("./scripts/init.lua").unwrap();
    state.context(move |ctx| {
        let send_to_sender = listener_sender.clone();
        let send_to = ctx
            .create_function(move |_, (id, line): (usize, String)| {
                send_to_sender.try_send(Command::SendTo(id, line)).unwrap();
                Ok(())
            })
            .unwrap();

        let send_to_all_sender = listener_sender.clone();
        let send_to_all = ctx
            .create_function(move |_, line: String| {
                send_to_all_sender
                    .try_send(Command::SendToAll(line))
                    .unwrap();
                Ok(())
            })
            .unwrap();

        let kick_sender = listener_sender.clone();
        let kick = ctx
            .create_function(move |_, id: usize| {
                kick_sender.try_send(Command::Kick(id)).unwrap();
                Ok(())
            })
            .unwrap();

        let kick_all_sender = listener_sender.clone();
        let kick_all = ctx
            .create_function(move |_, ()| {
                kick_all_sender.try_send(Command::KickAll).unwrap();
                Ok(())
            })
            .unwrap();

        let shutdown_sender = listener_sender.clone();
        let shutdown = ctx
            .create_function(move |ctx, ()| {
                let globals = ctx.globals();
                let modules: Table = globals.get("modules").unwrap();
                let server: Table = modules.get("server").unwrap();
                let running = server.get("running").unwrap();

                if running {
                    shutdown_sender.try_send(Command::Shutdown).unwrap();
                    server.set("running", false).unwrap();
                }

                Ok(())
            })
            .unwrap();

        let server = ctx.create_table().unwrap();
        server.set("running", true).unwrap();
        server.set("send_to", send_to).unwrap();
        server.set("send_to_all", send_to_all).unwrap();
        server.set("kick", kick).unwrap();
        server.set("kick_all", kick_all).unwrap();
        server.set("shutdown", shutdown).unwrap();

        let modules: Table = ctx.globals().get("modules").unwrap();
        modules.set("server", server).unwrap();

        ctx.load(code.as_bytes()).exec().unwrap();
    });

    state.context(move |ctx| {
        let globals = ctx.globals();
        let modules: Table = globals.get("modules").unwrap();
        let server: Table = modules.get("server").unwrap();
        let on_initialize: Function = server.get("on_initialize").unwrap();
        on_initialize.call::<_, ()>(()).unwrap();
    });
}
