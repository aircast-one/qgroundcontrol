local here = string.match(arg[0], "(.*/)") or "./"

local source = assert(io.open(here .. "aircast-usb.lua")):read("*a")
for _, operator in ipairs({ "//", "&", "<<", ">>" }) do
  assert(not string.find(source, operator, 1, true), "EdgeTX 2.9 runs Lua 5.2, which has no " .. operator .. " operator")
end

local EXIT = 1537

getmetatable("").__index = nil

local function radio(screen)
  local sent, drawn, pages, inserted = {}, {}, {}, {}
  local controls = { sf = 105, se = 104, rs = 99, ls = 98, s2 = 97, sg = 106, sh = 107, sb = 101, ["6pos"] = 108, s1 = 96, sc = 103 }
  local clock, inbox, cable = 0, "", true
  local env = setmetatable({
    SMLSIZE = 0, MIDSIZE = 0, INVERS = 0,
    LCD_W = screen.w, LCD_H = screen.h,
    EVT_VIRTUAL_EXIT = EXIT,
    getTime = function() return clock end,
    getOutputValue = function(index) return ({ [0] = -1024, [1] = 1024, [3] = -512, [10] = -1024, [17] = 1024 })[index] or 0 end,
    getValue = function() return 8.3 end,
    getSourceName = function(source) return "src" .. source end,
    getFieldInfo = function(name) return controls[name] and { id = controls[name] } end,
    setSerialBaudrate = function(baud) assert(baud == 115200) end,
    serialWrite = function(data) sent[#sent + 1] = data end,
    serialRead = function() local got = inbox; inbox = ""; return got end,
    usbConnected = function() return cable end,
    model = {
      getInfo = function() return { name = "VAMPIRE" } end,
      getMix = function(channel) return inserted[channel] or (channel < (screen.mixes or 4) and { name = "m" .. channel, source = channel } or nil) end,
      insertMix = function(channel, line, mix) assert(line == 0); inserted[channel] = mix end,
    },
    lcd = setmetatable({ RGB = function() return 0 end, clear = function() for k in pairs(drawn) do drawn[k] = nil end end },
      { __index = function(_, name) return function(_, _, text) drawn[#drawn + 1] = { name, text } end end }),
  }, { __index = _G })
  if screen.lvgl then
    env.lvgl = { clear = function() for k in pairs(drawn) do drawn[k] = nil end end, page = function(page)
      pages[#pages + 1] = page
      drawn[#drawn + 1] = { "page", page.title }
      return setmetatable({}, { __index = function(_, name) return function(_, spec)
        drawn[#drawn + 1] = { name, spec.text, spec.y }
        if spec.size then spec.size() end
      end end })
    end }
  end
  local script = assert(loadfile(here .. "aircast-usb.lua", "t", env))()
  script.init()
  return {
    script = script, sent = sent, drawn = drawn, pages = pages, inserted = inserted,
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
  assert(#r.sent == 1, "each run streams one line: twenty identical ones could overrun the radio's 2 KB USB buffer")
  local values = {}
  for value in string.gmatch(r.sent[1], "(-?%d+),") do values[#values + 1] = tonumber(value) end
  assert(string.sub(r.sent[1], 1, 1) == "$" and string.sub(r.sent[1], -2) == "\r\n", "the R2D2 line format the app and R2D2 both read")
  assert(#values == 24 and values[1] == -1024 and values[2] == 1024 and values[4] == -512 and values[11] == -1024 and values[18] == 1024)

  if screen.w == 480 then
    assert(shows(r, "Waiting for the app") and shows(r, "Pick it under Radio on USB"), "silence says where in the app to pick the radio")
    assert(shows(r, "CH1  src0  m0      988") and shows(r, "CH4  src3  m3     1244"), "each row names the channel the app receives, the stick on it and the mix's name")
    assert(shows(r, "+20 unused channels") and not shows(r, "CH24"), "only the channels this model mixes are listed")
    assert(next(r.inserted) == nil, "nothing is set up before the app says which channels it reads")
    assert(shows(r, "RTN: stop sending sticks and exit"), "the way out is on the screen")

    r.hear("@F,1\n")
    r.script.run(0)
    assert(shows(r, "App flying vehicle 1"), "the app's echo says the sticks reach a vehicle")
    assert(not shows(r, "Pick it under Radio on USB"), "a hearing app needs no instructions")
    r.hear("@S,2\n@N")
    r.script.run(0)
    assert(shows(r, "App: network standby, vehicle 2"), "a half-sent line waits for its newline")

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
full.hear("@F,1,C,1,2,3,4;18,20,21,22,24,11,12,5,19,23\n")
full.script.run(0)
assert(next(full.inserted) == nil, "a channel that already has a mix keeps it: nothing is added to a model mixing all 24")
local fresh = radio({ w = 480, h = 272 })
fresh.hear("@F,1\n")
fresh.script.run(0)
local wanted = { [10] = { "skld", 105 }, [11] = { "BOMARM", 104 }, [17] = { "camREB", 99 }, [19] = { "camyaw", 98 }, [20] = { "zoom", 97 }, [21] = { "camtyp", 106 }, [23] = { "termty", 107 }, [18] = { "campit", 96 }, [22] = { "cam90", 103 } }
for channel, mix in pairs(wanted) do
  local got = fresh.inserted[channel]
  assert(got and got.name == mix[1] and got.source == mix[2] and got.weight == 100, "an empty CH" .. (channel + 1) .. " gets the R2D2 " .. mix[1] .. " mix")
end
local count = 0
for _ in pairs(fresh.inserted) do count = count + 1 end
assert(count == 9, "an app that names no layout gets the R2D2 camera and payload channels, S1 and SC included, never the bay on CH5, which can be flight modes")
local moved = radio({ w = 480, h = 272 })
moved.hear("@F,1,C,1,2,3,4;13,14,15,0,0,0,0,0,0,0\n")
moved.script.run(0)
assert(moved.inserted[12] and moved.inserted[12].name == "camREB" and moved.inserted[12].source == 99, "the app's pitch channel gets the right slider")
assert(moved.inserted[13] and moved.inserted[13].name == "camyaw" and moved.inserted[14] and moved.inserted[14].name == "zoom", "and its yaw and zoom channels the left slider and S2")
count = 0
for _ in pairs(moved.inserted) do count = count + 1 end
assert(count == 3, "a function the app turned off (channel 0) is not set up anywhere")
local older = radio({ w = 480, h = 272 })
older.hear("@F,1,C,1,2,3,4,10;18,20,21,22,24,11,12,5\n")
older.script.run(0)
assert(not older.inserted[18] and not older.inserted[22], "an app naming only eight channels leaves S1 and SC alone, like any function its layout leaves out")
local onScreen = radio({ w = 480, h = 272, lvgl = true })
onScreen.hear("@F,1,C,1,2,3,4,10;18,20,21,22,24,11,12,5,19,23\n")
onScreen.script.run(0)
assert(shows(onScreen, string.format("CH%-2d %-5s %-6s %4d", 18, "src99", "camREB", 2012)), "a channel the script just set up appears on the screen")
local his = radio({ w = 480, h = 272 })
his.hear("@F,1,C,1,2,3,4,10;18,20,21,22,24,11,12,5,19,23\n")
his.script.run(0)
assert(his.inserted[9] and his.inserted[9].name == "phold" and his.inserted[9].source == 101, "a drone reading flight modes on CH10 gets SB there, like the R2D2 model")
assert(his.inserted[4] and his.inserted[4].source == 108, "and the six-position switch on CH5 picks the bay")
local stock = radio({ w = 480, h = 272 })
stock.hear("@F,1,C,1,2,3,4,5;18,20,21,22,24,11,12,0,19,23\n")
stock.script.run(0)
assert(stock.inserted[4] and stock.inserted[4].name == "phold", "a drone reading flight modes on CH5 gets SB there and no bay switch")
count = 0
for _ in pairs(stock.inserted) do count = count + 1 end
assert(count == 10, "the nine R2D2 functions and the flight mode switch")
local short = radio({ w = 480, h = 272 })
short.hear("@F,1,C,1,2,3,4;13\n")
short.script.run(0)
count = 0
for _ in pairs(short.inserted) do count = count + 1 end
assert(count == 1 and short.inserted[12], "a function the app's layout leaves out is off, not the R2D2 default")
moved.hear("@F,1,C,1,2,3,4;13,14,15,0,0,0,0,0\n")
moved.script.run(0)
moved.hear("@F,1,C,1,2,3,4;16,14,15,0,0,0,0,0\n")
moved.script.run(0)
assert(moved.inserted[15] and moved.inserted[15].name == "camREB", "moving pitch in the app sets the new channel up")
full.script.run(0)
assert(shows(full, "CH24 src23 m23    1500") and not shows(full, "+0 unused channels"), "a model mixing all 24 channels shows every one")

for _, mixes in ipairs({ 7, 24 }) do
  local tx16s = radio({ w = 480, h = 272, lvgl = true, mixes = mixes })
  tx16s.script.run(0)
  assert(shows(tx16s, string.format("CH%-2d %-5s %-6s %4d", mixes, "src" .. (mixes - 1), "m" .. (mixes - 1), 1500)))
  for _, call in ipairs(tx16s.drawn) do
    assert((call[3] or 0) <= 272 - 50 - 16, "a TX16S shows all " .. mixes .. " channels and the way out without scrolling")
  end
end

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
