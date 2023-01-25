pub mod server;

pub trait Module {
    fn register(&self, state: &rlua::Lua);
}
