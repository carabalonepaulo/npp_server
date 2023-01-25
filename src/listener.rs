use std::{process::Output, rc::Rc};

use async_std::{
    channel::{unbounded, Receiver, Sender},
    io::prelude::BufReadExt,
    io::BufReader,
    net::{Incoming, TcpListener, TcpStream},
    sync::Arc,
    sync::{Mutex, RwLock},
    task,
};
use futures::{
    channel::oneshot,
    io::{ReadHalf, WriteHalf},
    AsyncReadExt, AsyncWriteExt, Future, FutureExt, StreamExt,
};
use slab::Slab;

use crate::{
    events::{ListenerEvent, LuaEvent},
    generic_result::GenericResult,
};

enum ClientCommand {
    Send(String),
    Shutdown,
}

enum ListenerCommand {
    ClientConnected(oneshot::Sender<usize>, Sender<ClientCommand>),
    ClientDisconnected(usize),
    LineReceived(usize, String),
    SendTo(usize, String),
    SendToAll(String),
    Kick(usize),
    KickAll,
    Shutdown,
}

struct Shutdown;

#[derive(Debug)]
struct Trigger;

pub async fn run(sender: Sender<ListenerEvent>, mut receiver: Receiver<LuaEvent>) {
    println!("Listener started!");

    let (listener_command_sender, listener_command_receiver) = unbounded::<ListenerCommand>();

    task::spawn(command_handler(sender, listener_command_receiver));
    task::spawn(accept_loop(listener_command_sender.clone()));
    task::block_on(lua_event_handler(listener_command_sender, receiver));

    println!("Listener finalized.");
}

async fn lua_event_handler(sender: Sender<ListenerCommand>, receiver: Receiver<LuaEvent>) {
    let mut receiver = receiver.fuse();
    loop {
        let event_option = receiver.next().await;
        if event_option.is_none() {
            break;
        }

        match event_option.unwrap() {
            LuaEvent::SendTo(id, line) => sender
                .send(ListenerCommand::SendTo(id, line))
                .await
                .unwrap(),
            LuaEvent::SendToAll(line) => {
                sender.send(ListenerCommand::SendToAll(line)).await.unwrap()
            }
            LuaEvent::Kick(id) => sender.send(ListenerCommand::Kick(id)).await.unwrap(),
            LuaEvent::KickAll => sender.send(ListenerCommand::KickAll).await.unwrap(),
            LuaEvent::Shutdown => {
                sender.send(ListenerCommand::Shutdown).await.unwrap();
                break;
            }
        };
    }
}

async fn command_handler(
    listener_event_sender: Sender<ListenerEvent>,
    receiver: Receiver<ListenerCommand>,
) {
    let mut clients: Slab<Sender<ClientCommand>> = Slab::new();
    let mut receiver = receiver.fuse();

    loop {
        let command_option = receiver.next().await;
        if command_option.is_none() {
            break;
        }

        match command_option.unwrap() {
            ListenerCommand::ClientConnected(id_sender, client_command_sender) => {
                let entry = clients.vacant_entry();
                let id = entry.key();
                entry.insert(client_command_sender);

                id_sender.send(id).unwrap();
                listener_event_sender
                    .send(ListenerEvent::ClientConnected(id))
                    .await
                    .unwrap();
            }
            ListenerCommand::ClientDisconnected(id) => {
                clients.remove(id);
                listener_event_sender
                    .send(ListenerEvent::CliendDisconnected(id))
                    .await
                    .unwrap();
            }
            ListenerCommand::LineReceived(id, line) => {
                listener_event_sender
                    .send(ListenerEvent::LineReceived(id, line))
                    .await
                    .unwrap();
            }
            ListenerCommand::SendTo(id, line) => {
                if clients.contains(id) {
                    clients[id].send(ClientCommand::Send(line)).await.unwrap();
                }
            }
            ListenerCommand::SendToAll(line) => {
                for (key, sender) in clients.iter() {
                    sender
                        .send(ClientCommand::Send(line.clone()))
                        .await
                        .unwrap();
                }
            }
            ListenerCommand::Kick(id) => {
                clients[id].send(ClientCommand::Shutdown).await.unwrap();
            }
            ListenerCommand::KickAll => {
                for (_, sender) in clients.iter() {
                    sender.send(ClientCommand::Shutdown).await.unwrap();
                }
            }
            ListenerCommand::Shutdown => {
                for (_, sender) in clients.iter() {
                    sender.send(ClientCommand::Shutdown).await.unwrap();
                }
            }
        }
    }
}

async fn accept_loop(listener_command_sender: Sender<ListenerCommand>) {
    let listener = TcpListener::bind("127.0.0.1:5000").await.unwrap();
    let mut stream = listener.incoming().fuse();

    loop {
        let client_stream_option = stream.next().await;
        if client_stream_option.is_none() {
            break;
        }

        let client_stream_result = client_stream_option.unwrap();
        if client_stream_result.is_err() {
            continue;
        }

        let (reader, writer) = client_stream_result.unwrap().split();
        let (client_command_sender, client_command_receiver) = unbounded::<ClientCommand>();

        task::spawn(receive_loop(
            reader,
            listener_command_sender.clone(),
            client_command_sender.clone(),
        ));
        task::spawn(send_loop(writer, client_command_receiver));
    }
}

async fn send_loop(
    mut writer: WriteHalf<TcpStream>,
    client_command_receiver: Receiver<ClientCommand>,
) {
    let mut receiver = client_command_receiver.fuse();
    loop {
        match receiver.next().await {
            Some(command) => match command {
                ClientCommand::Send(mut line) => {
                    line.push('\n');
                    writer.write(line.as_bytes()).await;
                }
                ClientCommand::Shutdown => {
                    writer.close().await.unwrap();
                    break;
                }
            },
            None => break,
        }
    }
}

async fn receive_loop(
    reader: ReadHalf<TcpStream>,
    listener_command_sender: Sender<ListenerCommand>,
    client_command_sender: Sender<ClientCommand>,
) {
    let (id_sender, id_receiver) = oneshot::channel::<usize>();
    listener_command_sender
        .send(ListenerCommand::ClientConnected(
            id_sender,
            client_command_sender,
        ))
        .await
        .unwrap();

    let id = id_receiver.await.unwrap();
    let reader = BufReader::new(reader);
    let mut lines = reader.lines().fuse();

    loop {
        let line_option = lines.next().await;
        if line_option.is_none() {
            break;
        }

        let line_result = line_option.unwrap();
        if line_result.is_err() {
            break;
        }

        let line = line_result.unwrap();
        listener_command_sender
            .send(ListenerCommand::LineReceived(id, line))
            .await
            .unwrap();
    }

    listener_command_sender
        .send(ListenerCommand::ClientDisconnected(id))
        .await
        .unwrap();
}
