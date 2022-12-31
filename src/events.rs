#[derive(Debug)]
pub enum ListenerEvent {
    ClientConnected(usize),
    CliendDisconnected(usize),
    LineReceived(usize, String),
}

#[derive(Debug)]
pub enum LuaEvent {
    SendTo(usize, String),
    SendToAll(usize, String),
    Kick(usize),
    KickALl(usize),
    Shutdown,
}
