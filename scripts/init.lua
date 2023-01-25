local server = require 'server'
local packet = require 'scripts.packet'
local router = require 'scripts.router'

--[[
server.send_to(id, line)
server.send_to_all(line)
server.kick(id)
server.kick_all()
server.shutdown()
]]

function server.on_initialize()
  -- server.shutdown()
end

function server.on_finalize()
  error('not implemented yet')
end

function server.on_client_connected(id)
  print('client connected', id)
end

function server.on_client_disconnected(id)
  print('client disconnected', id)
end

function server.on_line_received(id, line)
  print('line received', id, line)
  router(id, line)
end

function server.on_tick(dt)
  error('not implemented yet')
end
