-- TNS|Aircast USB radio|TNE

local CHANNEL_COUNT = 24
local SERIAL_BAUDRATE = 115200
local SERIAL_PREFIX = "$"
local UPDATE_INTERVAL_TICKS = 10
local APP_SILENT_TICKS = 150
local CONFIRM_TICKS = 300
local SAME_PRESS_TICKS = 20
local BAR_MAX_W = 50
local COLUMN_ROWS = 12
local EXIT_HINT = "RTN: stop sending sticks and exit"
local CONFIRM_HINT = "Stop sending sticks? RTN again to exit"
local NO_CABLE = "USB cable not connected"
local NO_ANSWER = "Waiting for the app"
local NEXT_STEP = "In app: Control > Radio on USB"

local battVolt = 0
local modelInfo
local mixesInfo = {}
local channelValues = {}
local used = {}
local useLvglUI = false
local leaving = false
local confirmUntil = nil
local askedAt = nil
local heard = ""
local app = { state = nil, vehicle = nil, at = nil, labels = nil }

local ROLES = {
  C = { "Roll", "Pitch", "Thr", "Yaw" },
  P = { "Ail", "Ele", "Thr", "Rud" },
  R = { "Steer", false, "Thr", false },
}

local function labelsFor(kind, channels)
  local names = ROLES[kind]
  if not names then return nil end
  local labels = {}
  for at = 1, 4 do
    local channel = tonumber(channels[at])
    if names[at] and channel then labels[channel] = names[at] end
  end
  return labels
end

local SAYS = {
  F = function(vehicle) return "App flying vehicle " .. vehicle end,
  S = function(vehicle) return "App: 4G standby, vehicle " .. vehicle end,
  J = function(vehicle) return "A joystick is flying vehicle " .. vehicle end,
  N = function() return "App receiving, no vehicle connected" end,
}

local function readChannels()
  for i = 1, CHANNEL_COUNT do
    channelValues[i] = getOutputValue(i - 1)
  end
end

local function readMixes()
  used = {}
  for i = 1, CHANNEL_COUNT do
    local m = model.getMix(i - 1, 0)
    local srcName = ""
    if m and getSourceName and m.source then
      local ok, name = pcall(getSourceName, m.source)
      if ok and name then srcName = name end
    end
    mixesInfo[i] = { name = m and m.name or "", src = srcName }
    if m then used[#used + 1] = i end
  end
end

local function sendChannels()
  local data = SERIAL_PREFIX
  for i = 1, #channelValues do
    data = data .. channelValues[i] .. ","
  end
  serialWrite(data .. "\r\n")
end

local function stream()
  for _ = 1, UPDATE_INTERVAL_TICKS do
    readChannels()
    sendChannels()
  end
end

local function now()
  return getTime and getTime() or 0
end

local function listen()
  if not serialRead then return end
  heard = heard .. (serialRead() or "")
  local line = string.match(heard, "^(.-)\n")
  while line do
    heard = string.sub(heard, #line + 2)
    local state, vehicle = string.match(line, "^@(%a),?(%d*)")
    if state and SAYS[state] then
      local kind, roll, pitch, throttle, yaw = string.match(line, "^@%a,%d+,(%a),(%d+),(%d+),(%d+),(%d+)")
      app.state, app.vehicle, app.at = state, vehicle, now()
      app.labels = labelsFor(kind, { roll, pitch, throttle, yaw })
    end
    line = string.match(heard, "^(.-)\n")
  end
  if #heard > 256 then heard = "" end
end

local function appLive()
  return app.at ~= nil and now() - app.at <= APP_SILENT_TICKS
end

local function flying()
  return appLive() and (app.state == "F" or app.state == "S")
end

local function status()
  if appLive() then return SAYS[app.state](app.vehicle), true end
  if usbConnected and not usbConnected() then return NO_CABLE, false end
  return NO_ANSWER, false
end

local function nextStep()
  return status() == NO_ANSWER and NEXT_STEP or ""
end

local function confirming()
  return confirmUntil ~= nil and now() <= confirmUntil
end

local function hint()
  return confirming() and CONFIRM_HINT or EXIT_HINT
end

local function askToLeave()
  local t = now()
  if askedAt ~= nil and t - askedAt < SAME_PRESS_TICKS then return end
  askedAt = t
  if not flying() or confirming() then
    leaving = true
  else
    confirmUntil = t + CONFIRM_TICKS
  end
end

local function exitKey(event)
  return event ~= nil and event ~= 0 and (event == EVT_VIRTUAL_EXIT or event == EVT_EXIT_BREAK)
end

local function pwm(i)
  return math.floor(channelValues[i] / 2) + 1500
end

local function role(i)
  return appLive() and app.labels and app.labels[i] or nil
end

local function channelText(i)
  return string.format("CH%-2d %-5s %-6s %4d", i, role(i) or mixesInfo[i].src, mixesInfo[i].name, pwm(i))
end

local function place(slot)
  return (slot <= COLUMN_ROWS) and 1 or 2, (slot <= COLUMN_ROWS) and slot or (slot - COLUMN_ROWS)
end

local function unusedText()
  local unused = CHANNEL_COUNT - #used
  return unused > 0 and ("+" .. unused .. " unused channels") or ""
end

local function brand()
  return {
    background = lcd.RGB(0x0D, 0x0E, 0x11),
    foreground = lcd.RGB(0xFA, 0xFA, 0xFA),
    muted = lcd.RGB(0xD2, 0xD4, 0xD8),
    primary = lcd.RGB(0x00, 0x88, 0xF2),
    good = lcd.RGB(0x3D, 0xD6, 0x8C),
    warning = lcd.RGB(0xF4, 0xA4, 0x37),
  }
end

local function drawSmall()
  local said, ok = status()
  lcd.clear()
  lcd.drawText(0, 0, "AIRCAST", SMLSIZE + INVERS)
  lcd.drawText(40, 0, string.sub(modelInfo.name, 1, 8), SMLSIZE)
  lcd.drawText(92, 0, string.format("%1.1fV", battVolt), SMLSIZE)
  lcd.drawText(0, 8, string.sub(said, 1, 32), SMLSIZE + (ok and 0 or INVERS))
  lcd.drawFilledRectangle(0, 15, 128, 1)
  for slot = 1, math.min(#used, 15) do
    local i = used[slot]
    local x = math.floor((slot - 1) / 5) * 43
    local y = 17 + ((slot - 1) % 5) * 8
    lcd.drawText(x, y, string.sub(role(i) or ("CH" .. i), 1, 4), SMLSIZE)
    local w = math.floor((channelValues[i] + 1024 + 128) / 128)
    if w < 1 then w = 1 elseif w > 17 then w = 17 end
    lcd.drawFilledRectangle(x + 22, y + 1, w, 5)
  end
  lcd.drawText(0, 57, string.sub(hint(), 1, 32), SMLSIZE + (confirming() and INVERS or 0))
end

local function drawLarge()
  local color = brand()
  local said, ok = status()
  lcd.clear(color.background)
  lcd.drawText(10, 6, "Aircast USB", MIDSIZE + color.primary)
  lcd.drawText(170, 6, modelInfo.name, MIDSIZE + color.foreground)
  lcd.drawText(380, 6, string.format("%.2fV", battVolt), MIDSIZE + color.foreground)
  lcd.drawText(10, 36, said, MIDSIZE + (ok and color.good or color.warning))
  lcd.drawText(270, 42, nextStep(), SMLSIZE + color.warning)
  for slot = 1, #used do
    local i = used[slot]
    local column, row = place(slot)
    local colBase = column == 1 and 10 or 245
    local y = 50 + row * 16
    lcd.drawText(colBase, y, channelText(i), SMLSIZE + color.foreground)
    lcd.drawFilledRectangle(colBase + 170, y + 4, math.floor((channelValues[i] + 1024) * 55 / 2048) + 1, 6, color.primary)
  end
  lcd.drawText(10, 256, hint(), SMLSIZE + (confirming() and color.warning or color.muted))
  lcd.drawText(330, 256, unusedText(), SMLSIZE + color.muted)
end

local function buildLvglUI()
  local color = brand()
  local pg = lvgl.page({
    title = "Aircast USB",
    subtitle = modelInfo.name,
    back = askToLeave,
  })

  pg:label({ x = 0, y = 0, font = MIDSIZE,
    text = function() return (status()) end,
  })
  pg:label({ x = 330, y = 0, font = MIDSIZE,
    text = function() return string.format("%.1fV", battVolt) end,
  })
  pg:label({ x = 0, y = 30, font = SMLSIZE, color = color.warning, text = nextStep })

  for slot = 1, #used do
    local i = used[slot]
    local column, row = place(slot)
    local colBase = column == 1 and 0 or 235
    local yOfs = 30 + row * 22

    pg:label({ x = colBase, y = yOfs, font = SMLSIZE,
      text = function() return channelText(i) end,
    })

    pg:rectangle({ x = colBase + 175, y = yOfs + 6, w = BAR_MAX_W, h = 8, filled = true,
      color = color.primary,
      size = function()
        local w = math.floor((channelValues[i] + 1024) * BAR_MAX_W / 2048)
        if w < 1 then w = 1 elseif w > BAR_MAX_W then w = BAR_MAX_W end
        return w, 8
      end,
    })
  end
  local column, row = place(#used + 1)
  pg:label({ x = column == 1 and 0 or 235, y = 30 + row * 22, font = SMLSIZE, color = color.muted, text = unusedText() })
  pg:label({ x = 0, y = 30 + (COLUMN_ROWS + 2) * 22, font = SMLSIZE, text = hint })
end

local function init()
  modelInfo = model.getInfo() or { name = "Unknown" }
  readMixes()
  readChannels()
  setSerialBaudrate(SERIAL_BAUDRATE)
  if lvgl then
    useLvglUI = true
    buildLvglUI()
  end
end

local function run(event)
  listen()
  if exitKey(event) then askToLeave() end
  if leaving then return 1 end
  battVolt = getValue("tx-voltage")
  stream()
  if not useLvglUI then
    if LCD_W >= 480 and LCD_H >= 272 then
      drawLarge()
    elseif LCD_W == 128 and LCD_H == 64 then
      drawSmall()
    end
  end
  stream()
  return 0
end

return { run = run, init = init, useLvgl = true }
