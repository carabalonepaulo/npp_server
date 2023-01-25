local server = require 'server'
local packet = require 'scripts.packet'

return function(sender_id, tag, content)
  local name, password = content:match('(%w+):(%w+)')
  print(name, password)
  server.shutdown()
end
