local here = string.match(arg[0], "(.*/)") or "./"

local source = assert(io.open(here .. "aircast-usb.lua")):read("*a")
for _, operator in ipairs({ "//", "&", "<<", ">>" }) do
  assert(not string.find(source, operator, 1, true), "EdgeTX 2.9 runs Lua 5.2, which has no " .. operator .. " operator")
end

local EXIT = 1537

getmetatable("").__index = nil

local function radio(screen)
  local sent, drawn, pages = {}, {}, {}
  local clock, inbox, cable = 0, "", true
  local env = setmetatable({
    SMLSIZE = 0, MIDSIZE = 0, INVERS = 0,
    LCD_W = screen.w, LCD_H = screen.h,
    EVT_VIRTUAL_EXIT = EXIT,
    getTime = function() return clock end,
    getOutputValue = function(index) return ({ [0] = -1024, [1] = 1024, [3] = -512, [10] = -1024, [17] = 1024 })[index] or 0 end,
    getValue = function() return 8.3 end,
    getSourceName = function(source) return "src" .. source end,
    setSerialBaudrate = function(baud) assert(baud == 115200) end,
    serialWrite = function(data) sent[#sent + 1] = data end,
    serialRead = function() local got = inbox; inbox = ""; return got end,
    usbConnected = function() return cable end,
    model = {
      getInfo = function() return { name = "VAMPIRE" } end,
      getMix = function(channel) return channel < (screen.mixes or 4) and { name = "m" .. channel, source = channel } or nil end,
    },
    lcd = setmetatable({ RGB = function() return 0 end, clear = function() for k in pairs(drawn) do drawn[k] = nil end end },
      { __index = function(_, name) return function(_, _, text) drawn[#drawn + 1] = { name, text } end end }),
  }, { __index = _G })
  if screen.lvgl then
    env.lvgl = { page = function(page)
      pages[#pages + 1] = page
      drawn[#drawn + 1] = { "page", page.title }
      return setmetatable({}, { __index = function(_, name) return function(_, spec)
        drawn[#drawn + 1] = { name, spec.text }
        if spec.size then spec.size() end
      end end })
    end }
  end
  local script = assert(loadfile(here .. "aircast-usb.lua", "t", env))()
  script.init()
  return {
    script = script, sent = sent, drawn = drawn, pages = pages,
    at = function(t) clock = t end,
    hear = function(line) inbox = inbox .. line end,
    unplug = function() cable = false end,
  }
end

local function shows(r, text)
  for _, call in ipairs(r.drawn) do
    local said = type(call[2]) == "function" and call[2]() or call[2]
    if said == text then return true end
  end
  return false
end

for _, screen in ipairs({ { w = 480, h = 272 }, { w = 480, h = 320, lvgl = true }, { w = 128, h = 64 } }) do
  local r = radio(screen)
  assert(r.script.run() == 0)
  assert(#r.sent == 20, "each run streams 20 frames")
  local values = {}
  for value in string.gmatch(r.sent[1], "(-?%d+),") do values[#values + 1] = tonumber(value) end
  assert(string.sub(r.sent[1], 1, 1) == "$" and string.sub(r.sent[1], -2) == "\r\n", "the R2D2 line format the app and R2D2 both read")
  assert(#values == 24 and values[1] == -1024 and values[2] == 1024 and values[4] == -512 and values[11] == -1024 and values[18] == 1024)

  if screen.w == 480 then
    assert(shows(r, "Waiting for the app") and shows(r, "In app: Control > Radio on USB"), "silence says where in the app to pick the radio")
    assert(shows(r, "CH1  src0  m0      988") and shows(r, "CH4  src3  m3     1244"), "each row names the channel the app receives, the stick on it and the mix's name")
    assert(shows(r, "+20 unused channels") and not shows(r, "CH24"), "only the channels this model mixes are listed")
    assert(shows(r, "RTN: stop sending sticks and exit"), "the way out is on the screen")

    r.hear("@F,1\n")
    r.script.run(0)
    assert(shows(r, "App flying vehicle 1"), "the app's echo says the sticks reach a vehicle")
    assert(not shows(r, "In app: Control > Radio on USB"), "a hearing app needs no instructions")
    r.hear("@S,2\n@N")
    r.script.run(0)
    assert(shows(r, "App: 4G standby, vehicle 2"), "a half-sent line waits for its newline")

    r.at(151)
    r.script.run(0)
    assert(shows(r, "Waiting for the app"), "an app silent for 1.5 s is not receiving")
    r.unplug()
    r.script.run(0)
    assert(shows(r, "USB cable not connected"))
  else
    assert(shows(r, "AIRCAST"))
  end
end

local full = radio({ w = 480, h = 272, mixes = 24 })
full.script.run(0)
assert(shows(full, "CH24 src23 m23    1500") and not shows(full, "+0 unused channels"), "a model mixing all 24 channels shows every one")

local labelled = radio({ w = 480, h = 272 })
labelled.hear("@F,1,C,2,1,3,4\n")
labelled.script.run(0)
assert(shows(labelled, "CH1  Pitch m0      988") and shows(labelled, "CH2  Roll  m1     2012") and shows(labelled, "CH3  Thr   m2     1500") and shows(labelled, "CH4  Yaw   m3     1244"), "a copter's channels are named by what its RCMAP makes them do")
labelled.hear("@F,1,P,1,2,3,4\n")
labelled.script.run(0)
assert(shows(labelled, "CH1  Ail   m0      988") and shows(labelled, "CH4  Rud   m3     1244"), "a plane keeps plane names")
labelled.hear("@S,1,R,1,2,3,4\n")
labelled.script.run(0)
assert(shows(labelled, "CH1  Steer m0      988") and shows(labelled, "CH2  src1  m1     2012") and shows(labelled, "CH3  Thr   m2     1500"), "a rover names steering and throttle and leaves the rest to EdgeTX")
labelled.hear("@F,1\n")
labelled.script.run(0)
assert(shows(labelled, "CH1  src0  m0      988"), "an app that does not know the vehicle leaves EdgeTX's names")
labelled.hear("@F,1,C,1,2,3,4\n")
labelled.script.run(0)
labelled.at(151)
labelled.script.run(0)
assert(shows(labelled, "CH1  src0  m0      988"), "a silent app's labels fall back to EdgeTX's names")

local small = radio({ w = 128, h = 64 })
small.hear("@F,1,C,1,2,3,4\n")
small.script.run(0)
assert(shows(small, "Roll") and shows(small, "Pitc"), "the small screen shows the role, cut to fit")

local colourLabels = radio({ w = 480, h = 320, lvgl = true })
colourLabels.hear("@F,1,C,1,2,3,4\n")
colourLabels.script.run(0)
assert(shows(colourLabels, "CH1  Roll  m0      988"), "an LVGL radio labels its rows the same way")

local quiet = radio({ w = 480, h = 272 })
quiet.script.run(0)
assert(quiet.script.run(EXIT) == 1, "with nothing flying, one RTN leaves")

local live = radio({ w = 480, h = 272 })
live.hear("@F,1\n")
live.script.run(0)
local streamed = #live.sent
assert(live.script.run(EXIT) == 0, "while the app flies on these sticks, one RTN only asks")
assert(shows(live, "Stop sending sticks? RTN again to exit"))
assert(#live.sent > streamed, "asking does not stop the sticks")
live.at(100)
live.hear("@F,1\n")
assert(live.script.run(EXIT) == 1, "a second RTN within 3 s leaves")

local lapsed = radio({ w = 480, h = 272 })
lapsed.hear("@F,1\n")
lapsed.script.run(0)
lapsed.script.run(EXIT)
lapsed.at(140)
lapsed.hear("@F,1\n")
lapsed.at(350)
lapsed.hear("@F,1\n")
assert(lapsed.script.run(EXIT) == 0, "an RTN long after the question asks again")

local colour = radio({ w = 480, h = 320, lvgl = true })
colour.hear("@F,1\n")
colour.script.run(0)
colour.pages[1].back()
assert(colour.script.run(0) == 0, "the page's back button asks first while flying")
colour.at(50)
colour.pages[1].back()
assert(colour.script.run(0) == 1, "and leaves on the second press")

local both = radio({ w = 480, h = 320, lvgl = true })
both.hear("@F,1\n")
both.script.run(0)
both.pages[1].back()
assert(both.script.run(EXIT) == 0, "one RTN that reaches both the page's back and run is still one press, so it only asks")
both.at(50)
assert(both.script.run(EXIT) == 1, "and a real second press leaves")

print("aircast-usb.lua ok")
