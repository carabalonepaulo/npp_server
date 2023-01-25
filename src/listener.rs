use async_std::{
    channel::{unbounded, Receiver, Sender},
    io::prelude::BufReadExt,
    io::BufReader,
    net::{TcpListener, TcpStream},
    task,
};
use futures::{
    channel::oneshot,
    io::{ReadHalf, WriteHalf},
    AsyncReadExt, AsyncWriteExt, StreamExt,
};
use slab::Slab;

use crate::lua;

pub enum ClientCommand {
    Send(String),
    Shutdown,
}

pub enum Command {
    ClientConnected(oneshot::Sender<usize>, Sender<ClientCommand>),
    ClientDisconnected(usize),
    LineReceived(usize, String),

    SendTo(usize, String),
    SendToAll(String),
    Kick(usize),
    KickAll,
    Shutdown,
}

#[derive(Debug)]
struct Trigger;

pub async fn run(
    lua_sender: Sender<lua::Command>,
    listener_sender: Sender<Command>,
    listener_receiver: Receiver<Command>,
) {
    task::spawn(accept_loop(listener_sender));
    task::block_on(command_handler(lua_sender, listener_receiver.clone()));
}

async fn command_handler(lua_sender: Sender<lua::Command>, receiver: Receiver<Command>) {
    let mut clients: Slab<Sender<ClientCommand>> = Slab::new();
    let mut receiver = receiver.fuse();

    loop {
        let command_option = receiver.next().await;
        if command_option.is_none() {
            break;
        }

        match command_option.unwrap() {
            Command::ClientConnected(id_sender, client_command_sender) => {
                let entry = clients.vacant_entry();
                let id = entry.key();
                entry.insert(client_command_sender);

                id_sender.send(id).unwrap();
                lua_sender
                    .send(lua::Command::ClientConnected(id))
                    .await
                    .unwrap();
            }
            Command::ClientDisconnected(id) => {
                clients.remove(id);
                lua_sender
                    .send(lua::Command::ClientDisconnected(id))
                    .await
                    .unwrap();
            }
            Command::LineReceived(id, line) => {
                lua_sender
                    .send(lua::Command::LineReceived(id, line))
                    .await
                    .unwrap();
            }
            Command::SendTo(id, line) => {
                if clients.contains(id) {
                    clients[id].send(ClientCommand::Send(line)).await.unwrap();
                }
            }
            Command::SendToAll(line) => {
                for (_, sender) in clients.iter() {
                    sender
                        .send(ClientCommand::Send(line.clone()))
                        .await
                        .unwrap();
                }
            }
            Command::Kick(id) => {
                clients[id].send(ClientCommand::Shutdown).await.unwrap();
            }
            Command::KickAll => {
                for (_, sender) in clients.iter() {
                    sender.send(ClientCommand::Shutdown).await.unwrap();
                }
            }
            Command::Shutdown => {
                for (_, sender) in clients.iter() {
                    sender.send(ClientCommand::Shutdown).await.unwrap();
                }
                lua_sender.send(lua::Command::Shutdown).await.unwrap();
                break;
            }
        }
    }
}

async fn accept_loop(listener_command_sender: Sender<Command>) {
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
                    writer.write(line.as_bytes()).await.unwrap();
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
    listener_command_sender: Sender<Command>,
    client_command_sender: Sender<ClientCommand>,
) {
    let (id_sender, id_receiver) = oneshot::channel::<usize>();
    listener_command_sender
        .send(Command::ClientConnected(id_sender, client_command_sender))
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
            .send(Command::LineReceived(id, line))
            .await
            .unwrap();
    }

    listener_command_sender
        .send(Command::ClientDisconnected(id))
        .await
        .unwrap();
}
