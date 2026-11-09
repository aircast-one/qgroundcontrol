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
local LVGL_NAME_X = 8
local LVGL_ACCENT_W = 3
local MOVED_RAW = 24
local MOVED_TICKS = 100
local LVGL_META_X = 72
local LVGL_BAR_X = 140
local LVGL_BAR_W = 40
local LVGL_PANEL_PAD = 6
local LVGL_STATUS_H = 42
local LVGL_BAR_H = 8
local LVGL_VALUE_X = 184
local EXIT_HINT = "RTN: stop sending sticks and exit"
local CONFIRM_HINT = "Stop sending sticks? RTN again to exit"
local NO_CABLE = "USB cable not connected"
local NO_ANSWER = "Waiting for the app"
local NEXT_STEP = "Pick it under Radio on USB"
local USB_MODE_STEP = "No offer in the app? USB mode must be Serial (VCP): replug and pick it"
local SILENT_CABLE_TICKS = 800
local ICON_PATH = "/SCRIPTS/TOOLS/aircast-usb.png"
local ICON_PNG = "\137PNG\013\010\026\010\000\000\000\013IHDR\000\000\000\030\000\000\000\030\008\004\000\000\000\1459f)\000\000\000\140IDAT8\203\237TA\014\192 \008[\205\254\255ev2\010\020P\147\221\236\201\205\150\022\156\131\200s\140v.\189\226m\188\209\0060\214\209q\194n\204\034\013_\194\136\189\159.f\216\227\177\211x\200QdR\244e.\229\0285\176Y\200\156Dt\027`Nlh\148\151M\219:9.\159vTR\191'\031\009\011h3$=\231)J\177u\225\172\182\034\141@/\134\008\192\207|\185\231\026\233}\174\226\023?\131\092\254_\236+\246\248\000\138\145D#\215\034\220G\000\000\000\000IEND\174B`\130"

local battVolt = 0
local modelInfo
local mixesInfo = {}
local channelValues = {}
local movedAt = {}
local cableSince = nil
local builtStep = ""
local lastValues = {}
local used = {}
local useLvglUI = false
local relayout = false
local leaving = false
local confirmUntil = nil
local askedAt = nil
local heard = ""
local app = { state = nil, vehicle = nil, at = nil, labels = nil, payload = nil, modes = {} }

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
  local t = getTime and getTime() or 0
  for i = 1, CHANNEL_COUNT do
    channelValues[i] = getOutputValue(i - 1)
    if lastValues[i] and math.abs(channelValues[i] - lastValues[i]) > MOVED_RAW then movedAt[i] = t end
    lastValues[i] = channelValues[i]
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
  { "sc", "cam90", 23, -100 },
  { "sd", "stkcam", 17 },
}
local BAY = 8
local layoutSetUp = nil
local layoutNames = {}
local appNamed = nil
local appMixes = {}
local unread = {}

local function layoutFrom(line)
  local tail = string.match(line, ";([%d,]+)")
  local modeChannel = tonumber(string.match(line, "^@%a,%d+,%a,%d+,%d+,%d+,%d+,(%d+)")) or 0
  local channels = {}
  for number in string.gmatch(tail or "", "(%d+)") do channels[#channels + 1] = tonumber(number) end
  local mixes = {}
  for at, wanted in ipairs(FUNCTIONS) do
    local channel = tail and (channels[at] or 0) or wanted[3]
    if at == BAY and modeChannel == 0 then channel = 0 end
    mixes[at] = { wanted[1], wanted[2], channel, wanted[4] }
  end
  if modeChannel > 0 then
    mixes[#mixes + 1] = { "sb", "phold", modeChannel }
  end
  return mixes, (tail or "R2D2") .. "@" .. modeChannel, tail and #channels or BAY
end

local function setUpChannels(mixes, named)
  appNamed, appMixes, layoutNames, unread = named, mixes, {}, {}
  for at, wanted in ipairs(FUNCTIONS) do
    if at > named and wanted[3] >= 1 then unread[wanted[3]] = true end
  end
  for _, wanted in ipairs(mixes) do
    local shown = wanted[2] ~= "" and wanted[2] or (wanted[1] == "6pos" and "bay" or "")
    if wanted[3] and wanted[3] >= 1 and shown ~= "" then layoutNames[wanted[3]] = shown end
  end
  relayout = useLvglUI
  if not (getFieldInfo and model.insertMix) then return end
  for _, wanted in ipairs(mixes) do
    local channel = wanted[3] or 0
    local field = getFieldInfo(wanted[1])
    if channel >= 1 and channel <= CHANNEL_COUNT and field and not model.getMix(channel - 1, 0) then
      model.insertMix(channel - 1, 0, { name = wanted[2], source = field.id, weight = wanted[4] or 100 })
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
      local payload, modes = string.match(line, "|(%a+)|([%w,]*)")
      app.payload, app.modes = payload, {}
      for name in string.gmatch(modes or "", "([%w]+)") do app.modes[#app.modes + 1] = name end
      local mixes, layout, named = layoutFrom(line)
      if layout ~= layoutSetUp then
        layoutSetUp = layout
        setUpChannels(mixes, named)
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
  if usbConnected and not usbConnected() then
    cableSince = nil
    return NO_CABLE, false
  end
  if cableSince == nil then cableSince = now() end
  return NO_ANSWER, false
end

local function nextStep()
  return status() == NO_ANSWER and NEXT_STEP or ""
end

local function secondStep()
  if status() ~= NO_ANSWER then return "" end
  return (cableSince ~= nil and now() - cableSince >= SILENT_CABLE_TICKS) and USB_MODE_STEP or ""
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
  cam90 = "Look", skld = "Drop", skid = "Drop", BOMARM = "Arm", phold = "Modes", bay = "Bay", stkcam = "Stick",
}

local function shownName(i)
  local name = mixesInfo[i].name
  return name ~= "" and name or layoutNames[i] or ""
end

local STICKS = { Ail = "Roll", Ele = "Pitch", Thr = "Thr", Rud = "Yaw" }

local function stickName(i)
  return STICKS[string.match(mixesInfo[i].src, "(%a+)$") or ""]
end

local function functionName(i)
  local name = shownName(i)
  return role(i) or stickName(i) or FRIENDLY[name] or (name ~= "" and name or mixesInfo[i].src)
end

local GROUP = {
  Pitch = "camera", Tilt = "camera", Yaw = "camera", Zoom = "camera", Picture = "camera", Palette = "camera", Look = "camera",
  Stick = "camera",
  Drop = "payload", Arm = "payload", Bay = "payload", Modes = "flight",
}

local function groupOf(i)
  if role(i) or stickName(i) or i <= 4 then return "flight" end
  return GROUP[FRIENDLY[shownName(i)] or ""] or "other"
end

local function percent(v)
  return string.format("%+d%%", math.floor(v * 100 / 1024 + 0.5))
end

local function share(v)
  return string.format("%d%%", math.floor((v + 1024) * 100 / 2048 + 0.5))
end

local function degrees(v)
  return string.format("%d\194\176", math.floor(-90 + (v + 1024) * 90 / 2048 + 0.5))
end

local VALUE = {
  Pitch = degrees, Tilt = degrees, Zoom = share, ["stick Thr"] = share,
  Picture = function(v) return v < -512 and "Day" or v > 512 and "IR" or "PiP" end,
  Palette = function(v) return v > 0 and "next" or "-" end,
  Look = function(v) return v > 512 and "ahead" or v < -512 and "down" or "-" end,
  Stick = function(v) return v > 0 and "camera" or "drone" end,
  Arm = function(v) return v > 512 and "ARMED" or "safe" end,
  Drop = function(v) return v > 512 and "DROP" or "-" end,
  Bay = function(v) return "bay " .. (math.floor((v + 1024) * 5 / 2048 + 0.5) + 1) end,
  Modes = function(v) return v < -512 and "up" or v > 512 and "down" or "mid" end,
}

local function modePosition(pwm)
  if pwm < 1231 then return 1 elseif pwm <= 1360 then return 2 elseif pwm <= 1490 then return 3
  elseif pwm <= 1620 then return 4 elseif pwm <= 1749 then return 5 else return 6 end
end

local function valueText(i)
  local key = role(i) and ("stick " .. role(i)) or functionName(i)
  if key == "Arm" and appLive() and app.payload then
    return (app.payload == "armed" or app.payload == "refused") and string.upper(app.payload) or app.payload
  end
  if key == "Modes" and appLive() and #app.modes > 0 then
    return app.modes[modePosition(pwm(i))] or VALUE.Modes(channelValues[i])
  end
  return (VALUE[key] or percent)(channelValues[i])
end

local function movedLately(i)
  return movedAt[i] ~= nil and now() - movedAt[i] < MOVED_TICKS
end

local function channelText(i)
  return string.format("CH%-2d %-5s %-6s %4d", i, role(i) or mixesInfo[i].src, shownName(i), pwm(i))
end

local function place(slot, rows)
  return (slot <= rows) and 1 or 2, (slot <= rows) and slot or (slot - rows)
end

local function mappedText()
  if not appNamed then return "" end
  local total, on = 0, 0
  for at, m in ipairs(appMixes) do
    if at <= appNamed and m[3] >= 1 then
      total = total + 1
      if mixesInfo[m[3]] and model.getMix(m[3] - 1, 0) then on = on + 1 end
    end
  end
  return string.format("%d of %d app functions on the radio", on, total)
end

local function hot(i)
  local name = functionName(i)
  if name == "Arm" and appLive() and app.payload then return app.payload == "armed" or app.payload == "refused" end
  return (name == "Arm" or name == "Drop") and channelValues[i] > 512
end

local function missingControls()
  if not appNamed then return {} end
  local missing = {}
  for at, m in ipairs(appMixes) do
    if at <= appNamed and m[3] >= 1 and not model.getMix(m[3] - 1, 0) then
      missing[#missing + 1] = { FRIENDLY[m[2]] or FRIENDLY[m[1] == "6pos" and "bay" or ""] or m[2], m[3] }
    end
  end
  return missing
end

local function shownOrder()
  local order = {}
  for _, i in ipairs(used) do
    if groupOf(i) ~= "other" then order[#order + 1] = i end
  end
  for _, i in ipairs(used) do
    if groupOf(i) == "other" then order[#order + 1] = i end
  end
  return order
end

local function brand()
  return {
    background = lcd.RGB(0x0D, 0x0E, 0x11),
    panel = lcd.RGB(0x17, 0x19, 0x1E),
    foreground = lcd.RGB(0xFA, 0xFA, 0xFA),
    muted = lcd.RGB(0x9A, 0xA0, 0xA8),
    primary = lcd.RGB(0x00, 0x88, 0xF2),
    good = lcd.RGB(0x3D, 0xD6, 0x8C),
    warning = lcd.RGB(0xF4, 0xA4, 0x37),
    track = lcd.RGB(0x2A, 0x2D, 0x34),
    camera = lcd.RGB(0x3D, 0xD6, 0x8C),
    payload = lcd.RGB(0xF4, 0xA4, 0x37),
    other = lcd.RGB(0x6B, 0x70, 0x78),
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
  lcd.drawText(10, 256, secondStep() ~= "" and secondStep() or hint(), SMLSIZE + ((confirming() or secondStep() ~= "") and color.warning or color.muted))
  lcd.drawText(300, 256, mappedText(), SMLSIZE + color.muted)
end

local function putIconOnCard()
  local f = io.open(ICON_PATH, "r")
  if f then
    local head = io.read(f, 8)
    io.close(f)
    if head == string.sub(ICON_PNG, 1, 8) then return ICON_PATH end
  end
  local w = io.open(ICON_PATH, "w")
  if not w then return nil end
  io.write(w, ICON_PNG)
  io.close(w)
  return ICON_PATH
end

local function iconOnCard()
  local ok, path = pcall(putIconOnCard)
  return ok and path or nil
end

local function buildLvglUI()
  local color = brand()
  local pg = lvgl.page({
    title = "Aircast USB",
    subtitle = modelInfo.name,
    icon = iconOnCard(),
    back = askToLeave,
  })

  local pageH = LCD_H - LVGL_HEADER_H
  local hintY = pageH - LVGL_LINE_H
  builtStep = nextStep() .. secondStep()
  local statusH = LVGL_STATUS_H + (nextStep() ~= "" and LVGL_LINE_H or 0) + (secondStep() ~= "" and LVGL_LINE_H or 0)
  local rowsTop = LVGL_ROWS_TOP + statusH - LVGL_STATUS_H
  local missing = missingControls()
  local rows = math.max(1, math.ceil((#used + #missing) / 2))
  local rowH = math.min(LVGL_ROW_H, math.floor((hintY - rowsTop - LVGL_PANEL_PAD) / rows))
  local rowFont = rowH >= LVGL_TEXT_ROW_H and 0 or SMLSIZE
  local panelW = math.floor(LCD_W / 2) - 6
  local panelH = rows * rowH + LVGL_PANEL_PAD

  pg:rectangle({ x = 0, y = 0, w = LCD_W, h = pageH, filled = true, color = color.background })
  pg:rectangle({ x = 4, y = 2, w = LCD_W - 8, h = statusH - 4, filled = true, rounded = 6, color = color.panel })
  pg:rectangle({ x = 12, y = 12, w = 10, h = 10, filled = true, rounded = 5, color = color.good,
    pos = function() local _, ok = status(); return ok and 12 or -20, 12 end,
  })
  pg:rectangle({ x = 12, y = 12, w = 10, h = 10, filled = true, rounded = 5, color = color.warning,
    pos = function() local _, ok = status(); return ok and -20 or 12, 12 end,
  })
  pg:label({ x = 30, y = 4, font = MIDSIZE, color = color.foreground,
    text = function() return (status()) end,
  })
  pg:label({ x = LCD_W - 76, y = 4, font = MIDSIZE, color = color.foreground,
    text = function() return string.format("%.1fV", battVolt) end,
  })
  if nextStep() ~= "" then
    pg:label({ x = 30, y = LVGL_STATUS_H - 10, font = SMLSIZE, color = color.warning, text = nextStep })
  end
  if secondStep() ~= "" then
    pg:label({ x = 30, y = LVGL_STATUS_H - 10 + LVGL_LINE_H, font = SMLSIZE, color = color.warning, text = secondStep })
  end
  pg:rectangle({ x = 4, y = rowsTop - LVGL_PANEL_PAD, w = panelW, h = panelH + LVGL_PANEL_PAD, filled = true, rounded = 6, color = color.panel })
  pg:rectangle({ x = LCD_W - 4 - panelW, y = rowsTop - LVGL_PANEL_PAD, w = panelW, h = panelH + LVGL_PANEL_PAD, filled = true, rounded = 6, color = color.panel })

  local order = shownOrder()
  for slot = 1, #order do
    local i = order[slot]
    local column, row = place(slot, rows)
    local colBase = column == 1 and 4 + LVGL_PANEL_PAD or LCD_W - panelW - 4 + LVGL_PANEL_PAD
    local yOfs = rowsTop + (row - 1) * rowH
    local barY = yOfs + math.floor((rowH - LVGL_BAR_H) / 2)
    local centre = colBase + LVGL_BAR_X + math.floor(LVGL_BAR_W / 2)

    local group = unread[i] and "other" or groupOf(i)
    local fill = ({ flight = color.primary, camera = color.camera, payload = color.payload, other = color.other })[group]

    pg:rectangle({ x = colBase, y = yOfs + 2, w = LVGL_ACCENT_W, h = rowH - 4, filled = true, color = fill,
      pos = function() return (movedLately(i) or hot(i)) and colBase or -LVGL_ACCENT_W - 1, yOfs + 2 end,
    })
    pg:label({ x = colBase + LVGL_NAME_X, y = yOfs, font = rowFont, color = group == "other" and color.muted or color.foreground,
      text = function() return functionName(i) end,
    })
    pg:label({ x = colBase + LVGL_META_X, y = yOfs + (rowFont == 0 and 3 or 0), font = SMLSIZE, color = color.muted,
      text = "CH" .. i .. " " .. mixesInfo[i].src,
    })
    pg:rectangle({ x = colBase + LVGL_BAR_X, y = barY, w = LVGL_BAR_W, h = LVGL_BAR_H, filled = true, rounded = 4,
      color = color.track,
    })
    pg:rectangle({ x = centre, y = barY, w = 1, h = LVGL_BAR_H, filled = true, rounded = 4,
      color = fill,
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
    if unread[i] then
      pg:label({ x = colBase + LVGL_VALUE_X, y = yOfs + (rowFont == 0 and 3 or 0), font = SMLSIZE, color = color.other, text = "not read" })
    else
      pg:label({ x = colBase + LVGL_VALUE_X, y = yOfs + (rowFont == 0 and 3 or 0), font = SMLSIZE, color = color.muted,
        text = function() return hot(i) and "" or valueText(i) end,
      })
      pg:label({ x = colBase + LVGL_VALUE_X, y = yOfs + (rowFont == 0 and 3 or 0), font = SMLSIZE, color = color.payload,
        text = function() return hot(i) and valueText(i) or "" end,
      })
    end
  end
  for at, gap in ipairs(missing) do
    local column, row = place(#order + at, rows)
    local colBase = column == 1 and 4 + LVGL_PANEL_PAD or LCD_W - panelW - 4 + LVGL_PANEL_PAD
    local yOfs = rowsTop + (row - 1) * rowH
    pg:label({ x = colBase + LVGL_NAME_X, y = yOfs, font = rowFont, color = color.muted, text = gap[1] })
    pg:label({ x = colBase + LVGL_META_X, y = yOfs + (rowFont == 0 and 3 or 0), font = SMLSIZE, color = color.warning, text = "no control on CH" .. gap[2] })
  end
  pg:label({ x = 4, y = hintY, font = SMLSIZE, color = color.muted,
    text = function() return confirming() and "" or hint() end,
  })
  pg:label({ x = 4, y = hintY, font = SMLSIZE, color = color.warning,
    text = function() return confirming() and hint() or "" end,
  })
  pg:label({ x = 270, y = hintY, font = SMLSIZE, color = color.muted, text = mappedText() })
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
  if useLvglUI and nextStep() .. secondStep() ~= builtStep then relayout = true end
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
