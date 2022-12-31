use async_std::{
    channel::{unbounded, Receiver, Sender},
    io::prelude::BufReadExt,
    io::BufReader,
    net::{TcpListener, TcpStream},
    sync::Arc,
    sync::Mutex,
    task,
};
use futures::{channel::oneshot, AsyncWriteExt, StreamExt};
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
}

pub async fn run(sender: Sender<ListenerEvent>, receiver: Receiver<LuaEvent>) -> GenericResult<()> {
    println!("Listener started!");

    let (listener_command_sender, listener_command_receiver) = unbounded::<ListenerCommand>();

    task::spawn(command_handler(sender, listener_command_receiver));
    task::spawn(accept_loop(listener_command_sender)).await;

    Ok(())
}

async fn command_handler(
    listener_event_sender: Sender<ListenerEvent>,
    mut receiver: Receiver<ListenerCommand>,
) -> GenericResult<()> {
    let mut clients: Slab<Sender<ClientCommand>> = Slab::new();

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
                    .await?;
            }
            ListenerCommand::ClientDisconnected(id) => {
                clients.remove(id);
                listener_event_sender
                    .send(ListenerEvent::CliendDisconnected(id))
                    .await?
            }
            ListenerCommand::LineReceived(id, line) => {
                listener_event_sender
                    .send(ListenerEvent::LineReceived(id, line))
                    .await?
            }
        }
    }

    Ok(())
}

async fn accept_loop(listener_command_sender: Sender<ListenerCommand>) -> GenericResult<()> {
    let listener = TcpListener::bind("127.0.0.1:5000").await?;
    let mut stream = listener.incoming();

    loop {
        let client_stream_option = stream.next().await;
        if client_stream_option.is_none() {
            break;
        }

        let client_stream_result = client_stream_option.unwrap();
        if client_stream_result.is_err() {
            continue;
        }

        let client = Arc::new(Mutex::new(client_stream_result.unwrap()));
        let (client_command_sender, client_command_receiver) = unbounded::<ClientCommand>();

        task::spawn(receive_loop(
            client.clone(),
            listener_command_sender.clone(),
            client_command_sender.clone(),
        ));
        task::spawn(send_loop(client, client_command_receiver));
    }

    Ok(())
}

async fn send_loop(
    client: Arc<Mutex<TcpStream>>,
    mut client_command_receiver: Receiver<ClientCommand>,
) -> GenericResult<()> {
    loop {
        match client_command_receiver.next().await {
            Some(command) => match command {
                ClientCommand::Send(line) => {
                    let mut stream = &*client.lock().await;
                    stream.write(line.as_bytes()).await?;
                }
                ClientCommand::Shutdown => {
                    let mut stream = &*client.lock().await;
                    stream.close().await?;
                    break;
                }
            },
            None => break,
        }
    }

    Ok(())
}

async fn receive_loop(
    client: Arc<Mutex<TcpStream>>,
    listener_command_sender: Sender<ListenerCommand>,
    client_command_sender: Sender<ClientCommand>,
) -> GenericResult<()> {
    let (id_sender, id_receiver) = oneshot::channel::<usize>();

    listener_command_sender
        .send(ListenerCommand::ClientConnected(
            id_sender,
            client_command_sender,
        ))
        .await?;

    let id = id_receiver.await?;
    let stream = &*client.lock().await;
    let reader = BufReader::new(stream);
    let mut lines = reader.lines();

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
            .await?;
    }

    listener_command_sender
        .send(ListenerCommand::ClientDisconnected(id))
        .await?;

    Ok(())
}
