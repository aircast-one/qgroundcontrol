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
  return (channelValues[i] // 2) + 1500
end

local function drawSmall()
  lcd.clear()
  lcd.drawText(0, 0, modelInfo.name, SMLSIZE)
  lcd.drawText(56, 1, "USB", SMLSIZE)
  lcd.drawText(84, 0, string.format("Batt: %1.1fV", battVolt), SMLSIZE)

  lcd.drawFilledRectangle(0, 7, 128, 1)
  lcd.drawFilledRectangle(53, 0, 22, 7)

  for i = 1, CHANNEL_COUNT do
    local x = ((i - 1) // 8) * 42
    local y = 9 + ((i - 1) % 8) * 7
    lcd.drawText(x, y, string.format("%02d    %s", i, string.sub(mixesInfo[i].src, 3)), SMLSIZE)
    local w = (channelValues[i] + 1024 + 256) // 255
    if w < 1 then w = 1 elseif w > 9 then w = 9 end
    lcd.drawFilledRectangle(x + 11, y, w, 6)
    lcd.drawPoint(x + 11 + 4, y + 2)
  end
end

local function drawLarge()
  local front = lcd.RGB(255, 255, 255)
  local fTitle = MIDSIZE + front
  local fSmall = SMLSIZE + front

  lcd.clear(lcd.RGB(0, 0, 0))
  lcd.drawText(10, 10, "Aircast USB", fTitle)
  lcd.drawText(240, 10, modelInfo.name, fTitle)
  lcd.drawText(240, 240, KEEP_OPEN, MIDSIZE + lcd.RGB(255, 165, 0))
  lcd.drawText(10, 240, string.format("RC BATTERY: %.2fV", battVolt), fTitle)

  for i = 1, CHANNEL_COUNT do
    local colBase = (i <= 12) and 10 or 240
    local row = (i <= 12) and i or (i - 12)
    local y = 30 + row * 16
    lcd.drawText(colBase, y, string.format("CH%02d:%04d", i, pwm(i)), fSmall)
    lcd.drawFilledRectangle(colBase, y + 14, (channelValues[i] + 1024 + 34) // 34, 2, front)
    lcd.drawText(colBase + 80, y, mixesInfo[i].src, fSmall)
    lcd.drawText(colBase + 140, y, mixesInfo[i].name, fSmall)
  end
end

local function buildLvglUI()
  local pg = lvgl.page({
    title = "Aircast USB",
    subtitle = modelInfo.name,
    back = function() return 2 end,
  })

  pg:label({ x = 0, y = 0, font = MIDSIZE,
    text = function() return string.format("Batt: %.1fV", battVolt) end,
  })
  pg:label({ x = 230, y = 0, font = MIDSIZE, color = COLOR_THEME_WARNING, text = KEEP_OPEN })

  for i = 1, CHANNEL_COUNT do
    local colBase = (i <= 12) and 0 or 230
    local row = (i <= 12) and i or (i - 12)
    local yOfs = 20 + row * 22

    pg:label({ x = colBase, y = yOfs, font = SMLSIZE,
      text = function() return string.format("CH%02d:%5d %s", i, pwm(i), mixesInfo[i].src) end,
    })

    pg:rectangle({ x = colBase, y = yOfs + 14, w = BAR_MAX_W, h = 3, filled = true,
      color = COLOR_THEME_PRIMARY1,
      size = function()
        local w = (channelValues[i] + 1024) * BAR_MAX_W // 2048
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

local function run()
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
