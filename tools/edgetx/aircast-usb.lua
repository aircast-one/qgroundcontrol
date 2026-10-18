-- TNS|Aircast USB radio|TNE

local CHANNEL_COUNT = 24
local SERIAL_BAUDRATE = 115200
local SERIAL_PREFIX = "$"
local UPDATE_INTERVAL_TICKS = 10
local APP_SILENT_TICKS = 150
local CONFIRM_TICKS = 300
local BAR_MAX_W = 60
local EXIT_HINT = "RTN: stop sending sticks and exit"
local CONFIRM_HINT = "Stop sending sticks? RTN again to exit"
local NO_CABLE = "USB cable not connected"
local NO_ANSWER = "No answer from the app: set Hardware > USB-VCP to LUA"

local battVolt = 0
local modelInfo
local mixesInfo = {}
local channelValues = {}
local used = {}
local useLvglUI = false
local leaving = false
local confirmUntil = nil
local heard = ""
local app = { state = nil, vehicle = nil, at = nil }

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
  local line = heard:match("^(.-)\n")
  while line do
    heard = heard:sub(#line + 2)
    local state, vehicle = line:match("^@(%a),?(%d*)")
    if state and SAYS[state] then
      app.state, app.vehicle, app.at = state, vehicle, now()
    end
    line = heard:match("^(.-)\n")
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

local function confirming()
  return confirmUntil ~= nil and now() <= confirmUntil
end

local function hint()
  return confirming() and CONFIRM_HINT or EXIT_HINT
end

local function askToLeave()
  if not flying() or confirming() then
    leaving = true
  else
    confirmUntil = now() + CONFIRM_TICKS
  end
end

local function exitKey(event)
  return event ~= nil and event ~= 0 and (event == EVT_VIRTUAL_EXIT or event == EVT_EXIT_BREAK)
end

local function pwm(i)
  return math.floor(channelValues[i] / 2) + 1500
end

local function afterList()
  local shown = math.min(#used, 16) + 1
  return (shown <= 8) and 1 or 2, (shown <= 8) and shown or (shown - 8)
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
    lcd.drawText(x, y, string.sub(mixesInfo[i].src ~= "" and mixesInfo[i].src or ("CH" .. i), 1, 4), SMLSIZE)
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
  lcd.drawText(10, 32, said, MIDSIZE + (ok and color.good or color.warning))
  for slot = 1, math.min(#used, 16) do
    local i = used[slot]
    local colBase = (slot <= 8) and 10 or 245
    local row = (slot <= 8) and slot or (slot - 8)
    local y = 46 + row * 22
    local label = mixesInfo[i].src ~= "" and mixesInfo[i].src or ("CH" .. i)
    lcd.drawText(colBase, y, string.format("%-6s %4d", label, pwm(i)), SMLSIZE + color.foreground)
    lcd.drawFilledRectangle(colBase + 100, y + 4, math.floor((channelValues[i] + 1024) * 100 / 2048) + 1, 6, color.primary)
  end
  local column, row = afterList()
  lcd.drawText(column == 1 and 10 or 245, 46 + row * 22, unusedText(), SMLSIZE + color.muted)
  lcd.drawText(10, 250, hint(), SMLSIZE + (confirming() and color.warning or color.muted))
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

  for slot = 1, math.min(#used, 16) do
    local i = used[slot]
    local colBase = (slot <= 8) and 0 or 230
    local row = (slot <= 8) and slot or (slot - 8)
    local yOfs = 10 + row * 24
    local label = mixesInfo[i].src ~= "" and mixesInfo[i].src or ("CH" .. i)

    pg:label({ x = colBase, y = yOfs, font = SMLSIZE,
      text = function() return string.format("%-6s %5d", label, pwm(i)) end,
    })

    pg:rectangle({ x = colBase, y = yOfs + 15, w = BAR_MAX_W, h = 3, filled = true,
      color = color.primary,
      size = function()
        local w = math.floor((channelValues[i] + 1024) * BAR_MAX_W / 2048)
        if w < 1 then w = 1 elseif w > BAR_MAX_W then w = BAR_MAX_W end
        return w, 3
      end,
    })
  end
  local column, row = afterList()
  pg:label({ x = column == 1 and 0 or 230, y = 10 + row * 24, font = SMLSIZE, color = color.muted, text = unusedText() })
  pg:label({ x = 0, y = 10 + 10 * 24, font = SMLSIZE, text = hint })
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
