local here = arg[0]:match("(.*/)") or "./"

local source = assert(io.open(here .. "aircast-usb.lua")):read("a")
for _, operator in ipairs({ "//", "&", "<<", ">>" }) do
  assert(not source:find(operator, 1, true), "EdgeTX 2.9 runs Lua 5.2, which has no " .. operator .. " operator")
end

local function radio(screen)
  local sent, drawn = {}, {}
  local env = setmetatable({
    SMLSIZE = 0, MIDSIZE = 0, INVERS = 0,
    LCD_W = screen.w, LCD_H = screen.h,
    getOutputValue = function(index) return ({ [0] = -1024, [1] = 1024, [3] = -512, [10] = -1024, [17] = 1024 })[index] or 0 end,
    getValue = function() return 8.3 end,
    getSourceName = function(source) return "src" .. source end,
    setSerialBaudrate = function(baud) assert(baud == 115200) end,
    serialWrite = function(data) sent[#sent + 1] = data end,
    model = {
      getInfo = function() return { name = "VAMPIRE" } end,
      getMix = function(channel) return channel < 4 and { name = "m" .. channel, source = channel } or nil end,
    },
    lcd = setmetatable({ RGB = function() return 0 end }, { __index = function(_, name) return function(_, _, text) drawn[#drawn + 1] = { name, text } end end }),
  }, { __index = _G })
  if screen.lvgl then
    env.lvgl = { page = function(page)
      drawn[#drawn + 1] = { "page", page.title }
      return setmetatable({}, { __index = function(_, name) return function(_, spec)
        drawn[#drawn + 1] = { name, type(spec.text) == "function" and spec.text() or spec.text }
        if spec.size then spec.size() end
      end end })
    end }
  end
  local script = assert(loadfile(here .. "aircast-usb.lua", "t", env))()
  script.init()
  assert(script.run() == 0)
  return sent, drawn
end

local function shown(drawn, text)
  for _, call in ipairs(drawn) do
    if call[2] == text then return true end
  end
  return false
end

for _, screen in ipairs({ { w = 480, h = 272 }, { w = 480, h = 320, lvgl = true }, { w = 128, h = 64 } }) do
  local sent, drawn = radio(screen)
  assert(#sent == 20, "each run streams 20 frames")
  local values = {}
  for value in sent[1]:gmatch("(-?%d+),") do values[#values + 1] = tonumber(value) end
  assert(sent[1]:sub(1, 1) == "$" and sent[1]:sub(-2) == "\r\n", "the R2D2 line format the app and R2D2 both read")
  assert(#values == 24 and values[1] == -1024 and values[2] == 1024 and values[4] == -512 and values[11] == -1024 and values[18] == 1024)
  assert(not shown(drawn, "R2D2 USB-VCP") and not shown(drawn, "TEST VERSION"))
  if screen.w == 480 then
    assert(shown(drawn, "Aircast USB") and shown(drawn, "Keep open while flying"), "a colour screen says to keep the script open")
  else
    assert(shown(drawn, "AIRCAST"))
  end
end

print("aircast-usb.lua ok")
