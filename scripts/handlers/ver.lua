local server = require 'server'

return function(sender_id)
  server.send_to(sender_id, '<0 ' .. tostring(sender_id) .. ">'e' n=Server</0>")
end
