--  The main script, do not tamper with if you have no knowledge of Lua
--  Copyright(c) 2006 sUiCiDeMAniC
--  Email:  manic15@gmail.com
--  Last update:  03/11/06

-- dofile( "./Scripts/functions.lua" )
require 'scripts.functions'

function StartUp()

end

function OnConnect()
  -- NP():SendToUser(user.id,
  --   "<chat>Hi and welcome to " ..
  --   NP():GetServerName() ..
  --   ", enjoy your stay " .. user.name .. "; Your ip address is: " ..
  --   user.ip .. " and your group is: " .. user.group .. "</chat>")
  -- StartNPCLoop()
end

function OnCMD(data, cmd)
  do process(cmd, data) end
end

function OnDisconnect()
  --NP():SendToAll( "<chat>The player "..user.name.." has left us, see ya!</chat>" )
end
