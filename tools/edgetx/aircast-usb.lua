-- TNS|Aircast USB radio|TNE

local CHANNEL_COUNT = 24
local SERIAL_BAUDRATE = 115200
local SERIAL_PREFIX = "$"
local UPDATE_INTERVAL_TICKS = 10
local KEEP_OPEN = "Keep open while flying"
local BAR_MAX_W = 60

local battVolt = 0
local modelInfo
local mixesInfo = {}
local channelValues = {}
local useLvglUI = false
local leaving = false

local function readChannels()
  for i = 1, CHANNEL_COUNT do
    channelValues[i] = getOutputValue(i - 1)
  end
end

local function readMixes()
  for i = 1, CHANNEL_COUNT do
    local m = model.getMix(i - 1, 0)
    local srcName = ""
    if m and getSourceName and m.source then
      local ok, name = pcall(getSourceName, m.source)
      if ok and name then srcName = name end
    end
    mixesInfo[i] = { name = m and m.name or "", src = srcName }
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

local function pwm(i)
  return math.floor(channelValues[i] / 2) + 1500
end

local function brand()
  return {
    background = lcd.RGB(0x0D, 0x0E, 0x11),
    foreground = lcd.RGB(0xFA, 0xFA, 0xFA),
    muted = lcd.RGB(0xD2, 0xD4, 0xD8),
    primary = lcd.RGB(0x00, 0x88, 0xF2),
    warning = lcd.RGB(0xF4, 0xA4, 0x37),
  }
end

local function drawSmall()
  lcd.clear()
  lcd.drawText(0, 0, "AIRCAST", SMLSIZE + INVERS)
  lcd.drawText(40, 0, string.sub(modelInfo.name, 1, 8), SMLSIZE)
  lcd.drawText(84, 0, string.format("Batt: %1.1fV", battVolt), SMLSIZE)
  lcd.drawFilledRectangle(0, 7, 128, 1)

  for i = 1, CHANNEL_COUNT do
    local x = math.floor((i - 1) / 8) * 42
    local y = 9 + ((i - 1) % 8) * 7
    lcd.drawText(x, y, string.format("%02d    %s", i, string.sub(mixesInfo[i].src, 3)), SMLSIZE)
    local w = math.floor((channelValues[i] + 1024 + 256) / 255)
    if w < 1 then w = 1 elseif w > 9 then w = 9 end
    lcd.drawFilledRectangle(x + 11, y, w, 6)
    lcd.drawPoint(x + 11 + 4, y + 2)
  end
end

local function drawLarge()
  local color = brand()
  local fSmall = SMLSIZE + color.foreground

  lcd.clear(color.background)
  lcd.drawText(10, 10, "Aircast USB", MIDSIZE + color.primary)
  lcd.drawText(240, 10, modelInfo.name, MIDSIZE + color.foreground)
  lcd.drawText(240, 240, KEEP_OPEN, MIDSIZE + color.warning)
  lcd.drawText(10, 240, string.format("RC BATTERY: %.2fV", battVolt), MIDSIZE + color.foreground)

  for i = 1, CHANNEL_COUNT do
    local colBase = (i <= 12) and 10 or 240
    local row = (i <= 12) and i or (i - 12)
    local y = 30 + row * 16
    lcd.drawText(colBase, y, string.format("CH%02d:%04d", i, pwm(i)), fSmall)
    lcd.drawFilledRectangle(colBase, y + 14, math.floor((channelValues[i] + 1024 + 34) / 34), 2, color.primary)
    lcd.drawText(colBase + 80, y, mixesInfo[i].src, SMLSIZE + color.muted)
    lcd.drawText(colBase + 140, y, mixesInfo[i].name, SMLSIZE + color.muted)
  end
end

local function buildLvglUI()
  local color = brand()
  local pg = lvgl.page({
    title = "Aircast USB",
    subtitle = modelInfo.name,
    back = function() leaving = true end,
  })

  pg:label({ x = 0, y = 0, font = MIDSIZE,
    text = function() return string.format("Batt: %.1fV", battVolt) end,
  })
  pg:label({ x = 230, y = 0, font = MIDSIZE, color = color.warning, text = KEEP_OPEN })

  for i = 1, CHANNEL_COUNT do
    local colBase = (i <= 12) and 0 or 230
    local row = (i <= 12) and i or (i - 12)
    local yOfs = 20 + row * 22

    pg:label({ x = colBase, y = yOfs, font = SMLSIZE,
      text = function() return string.format("CH%02d:%5d %s", i, pwm(i), mixesInfo[i].src) end,
    })

    pg:rectangle({ x = colBase, y = yOfs + 14, w = BAR_MAX_W, h = 3, filled = true,
      color = color.primary,
      size = function()
        local w = math.floor((channelValues[i] + 1024) * BAR_MAX_W / 2048)
        if w < 1 then w = 1 elseif w > BAR_MAX_W then w = BAR_MAX_W end
        return w, 3
      end,
    })
  end
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

local function exiting(event)
  return event ~= nil and event ~= 0 and (event == EVT_VIRTUAL_EXIT or event == EVT_EXIT_BREAK)
end

local function run(event)
  if leaving or exiting(event) then
    return 1
  end
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
