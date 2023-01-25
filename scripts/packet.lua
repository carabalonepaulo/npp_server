local server = require 'server'

return {
  parse = function(line)
    local index = line:find('>')
    return line:sub(1, index), line:sub(index + 1, line:find('</', index) - 1)
  end,

  send_to = function(id, tag, content)
    local end_tag = tag:sub(1, 1) .. '/' .. tag:sub(2, tag:len())
    print(tag .. content .. end_tag)
    server.send_to(id, tag .. content .. end_tag)
  end,

  send_to_all = function(tag, content)
    local end_tag = tag:sub(1, 1) .. '/' .. tag:sub(2, tag:len())
    server.send_to_all(tag .. content .. end_tag)
  end
}
