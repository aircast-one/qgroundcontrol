-- TNS|Aircast USB radio|TNE

local CHANNEL_COUNT = 24
local SERIAL_BAUDRATE = 115200
local SERIAL_PREFIX = "$"
local APP_SILENT_TICKS = 150
local CONFIRM_TICKS = 300
local SAME_PRESS_TICKS = 20
local COLUMN_ROWS = 12
local LVGL_HEADER_H = 50
local LVGL_ROWS_TOP = 46
local LVGL_ROW_H = 22
local LVGL_LINE_H = 16
local LVGL_TEXT_ROW_H = 17
local LVGL_META_X = 78
local LVGL_BAR_X = 156
local LVGL_BAR_W = 50
local LVGL_BAR_H = 8
local LVGL_VALUE_X = 210
local EXIT_HINT = "RTN: stop sending sticks and exit"
local CONFIRM_HINT = "Stop sending sticks? RTN again to exit"
local NO_CABLE = "USB cable not connected"
local NO_ANSWER = "Waiting for the app"
local NEXT_STEP = "Pick it under Radio on USB"

local battVolt = 0
local modelInfo
local mixesInfo = {}
local channelValues = {}
local used = {}
local useLvglUI = false
local relayout = false
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
  S = function(vehicle) return "App: network standby, vehicle " .. vehicle end,
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

local FUNCTIONS = {
  { "rs", "camREB", 18 },
  { "ls", "camyaw", 20 },
  { "s2", "zoom", 21 },
  { "sg", "camtyp", 22 },
  { "sh", "termty", 24 },
  { "sf", "skld", 11 },
  { "se", "BOMARM", 12 },
  { "6pos", "", 5 },
  { "s1", "campit", 19 },
  { "sc", "cam90", 23 },
}
local BAY = 8
local layoutSetUp = nil
local layoutNames = {}

local function layoutFrom(line)
  local tail = string.match(line, ";([%d,]+)")
  local modeChannel = tonumber(string.match(line, "^@%a,%d+,%a,%d+,%d+,%d+,%d+,(%d+)")) or 0
  local channels = {}
  for number in string.gmatch(tail or "", "(%d+)") do channels[#channels + 1] = tonumber(number) end
  local mixes = {}
  for at, wanted in ipairs(FUNCTIONS) do
    local channel = tail and (channels[at] or 0) or wanted[3]
    if at == BAY and modeChannel == 0 then channel = 0 end
    mixes[at] = { wanted[1], wanted[2], channel }
  end
  if modeChannel > 0 then
    mixes[#mixes + 1] = { "sb", "phold", modeChannel }
  end
  return mixes, (tail or "R2D2") .. "@" .. modeChannel
end

local function setUpChannels(mixes)
  layoutNames = {}
  for _, wanted in ipairs(mixes) do
    local shown = wanted[2] ~= "" and wanted[2] or (wanted[1] == "6pos" and "bay" or "")
    if wanted[3] and wanted[3] >= 1 and shown ~= "" then layoutNames[wanted[3]] = shown end
  end
  if not (getFieldInfo and model.insertMix) then return end
  for _, wanted in ipairs(mixes) do
    local channel = wanted[3] or 0
    local field = getFieldInfo(wanted[1])
    if channel >= 1 and channel <= CHANNEL_COUNT and field and not model.getMix(channel - 1, 0) then
      model.insertMix(channel - 1, 0, { name = wanted[2], source = field.id, weight = 100 })
    end
  end
  readMixes()
  relayout = useLvglUI
end

local function sendChannels()
  local data = SERIAL_PREFIX
  for i = 1, #channelValues do
    data = data .. channelValues[i] .. ","
  end
  serialWrite(data .. "\r\n")
end

local function stream()
  readChannels()
  sendChannels()
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
      local mixes, layout = layoutFrom(line)
      if layout ~= layoutSetUp then
        layoutSetUp = layout
        setUpChannels(mixes)
      end
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

local FRIENDLY = {
  camREB = "Pitch", campit = "Tilt", camyaw = "Yaw", zoom = "Zoom", camtyp = "Picture", termty = "Palette",
  cam90 = "Look", skld = "Drop", skid = "Drop", BOMARM = "Arm", phold = "Modes", bay = "Bay",
}

local function shownName(i)
  local name = mixesInfo[i].name
  return name ~= "" and name or layoutNames[i] or ""
end

local function functionName(i)
  local name = shownName(i)
  return role(i) or FRIENDLY[name] or (name ~= "" and name or mixesInfo[i].src)
end

local function channelText(i)
  return string.format("CH%-2d %-5s %-6s %4d", i, role(i) or mixesInfo[i].src, shownName(i), pwm(i))
end

local function place(slot, rows)
  return (slot <= rows) and 1 or 2, (slot <= rows) and slot or (slot - rows)
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
    track = lcd.RGB(0x2A, 0x2D, 0x34),
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
    local column, row = place(slot, COLUMN_ROWS)
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
  pg:label({ x = 0, y = 28, font = SMLSIZE, color = color.warning, text = nextStep })

  local hintY = LCD_H - LVGL_HEADER_H - LVGL_LINE_H
  local rows = math.max(1, math.ceil(#used / 2))
  local rowH = math.min(LVGL_ROW_H, math.floor((hintY - LVGL_ROWS_TOP) / rows))
  local rowFont = rowH >= LVGL_TEXT_ROW_H and 0 or SMLSIZE
  for slot = 1, #used do
    local i = used[slot]
    local column, row = place(slot, rows)
    local colBase = column == 1 and 0 or 245
    local yOfs = LVGL_ROWS_TOP + (row - 1) * rowH
    local barY = yOfs + math.floor((rowH - LVGL_BAR_H) / 2)
    local centre = colBase + LVGL_BAR_X + math.floor(LVGL_BAR_W / 2)

    pg:label({ x = colBase, y = yOfs, font = rowFont,
      text = function() return functionName(i) end,
    })
    pg:label({ x = colBase + LVGL_META_X, y = yOfs + (rowFont == 0 and 3 or 0), font = SMLSIZE, color = color.muted,
      text = "CH" .. i .. " " .. mixesInfo[i].src,
    })
    pg:rectangle({ x = colBase + LVGL_BAR_X, y = barY, w = LVGL_BAR_W, h = LVGL_BAR_H, filled = true, rounded = 4,
      color = color.track,
    })
    pg:rectangle({ x = centre, y = barY, w = 1, h = LVGL_BAR_H, filled = true, rounded = 4,
      color = color.primary,
      pos = function()
        local v = channelValues[i]
        return (v < 0) and (centre - math.floor(-v * LVGL_BAR_W / 2048)) or centre, barY
      end,
      size = function()
        local w = math.floor(math.abs(channelValues[i]) * LVGL_BAR_W / 2048)
        if w < 1 then w = 1 end
        return w, LVGL_BAR_H
      end,
    })
    pg:label({ x = colBase + LVGL_VALUE_X, y = yOfs + (rowFont == 0 and 3 or 0), font = SMLSIZE, color = color.muted,
      text = function() return string.format("%d", pwm(i)) end,
    })
  end
  pg:label({ x = 0, y = hintY, font = SMLSIZE, text = hint })
  pg:label({ x = 300, y = hintY, font = SMLSIZE, color = color.muted, text = unusedText() })
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
  if relayout then
    relayout = false
    lvgl.clear()
    buildLvglUI()
  end
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
