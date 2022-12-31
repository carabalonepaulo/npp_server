use async_std::{
    channel::{Receiver, Sender},
    stream::StreamExt,
};

use crate::{
    events::{ListenerEvent, LuaEvent},
    generic_result::GenericResult,
};

pub async fn run(
    sender: Sender<LuaEvent>,
    mut receiver: Receiver<ListenerEvent>,
) -> GenericResult<()> {
    println!("Lua started!");

    loop {
        if let Some(ev) = receiver.next().await {
            match ev {
                ListenerEvent::ClientConnected(id) => println!("Client #{id} connected!"),
                ListenerEvent::CliendDisconnected(id) => println!("Client #{id} disconnected!"),
                ListenerEvent::LineReceived(id, line) => {
                    println!("Line received from client #{id}: '{line}'")
                }
            }
        } else {
            break;
        }
    }
    Ok(())
}
