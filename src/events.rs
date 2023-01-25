#[derive(Debug)]
pub enum ListenerEvent {
    ClientConnected(usize),
    CliendDisconnected(usize),
    LineReceived(usize, String),
}

#[derive(Debug)]
pub enum LuaEvent {
    SendTo(usize, String),
    SendToAll(String),
    Kick(usize),
    KickAll,
    Shutdown,
}
