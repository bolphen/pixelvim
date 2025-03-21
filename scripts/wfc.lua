-- based on the WaveFunctionCollapse algorithm:
-- https://github.com/mxgmn/WaveFunctionCollapse
-- MIT License
-------------------------------------------------------------------------------
-- WFC algorithm implementation
-------------------------------------------------------------------------------
-- general implementation in lua, should be easily portable for other purposes
-- only the part of image manipulation (reading colors from input) needs
-- changing
-------------------------------------------------------------------------------
-- helper classes
local function grid(width, height, init)
  local instance = {width=width, height=height}
  instance.get = function(this, x, y)
    if 1 <= x and x <= this.width and 1 <= y and y <= this.height then
      return this[x + (y-1) * this.width]
    end
  end
  instance.set = function(this, x, y, value)
    if 1 <= x and x <= this.width and 1 <= y and y <= this.height then
      this[x + (y-1) * this.width] = value
    end
  end
  for i = 1, width*height do
    instance[i] = init or 0
  end
  return instance
end
local function indexedImage(width, height)
  local instance = grid(width, height)
  instance.getWindow = function(this, x, y, size)
    local window = grid(size, size)
    for i = 1, size do
      for j = 1, size do
        window:set(i, j, this:get(x+i-1, y+j-1))
      end
    end
    return window
  end
  instance.blit = function(this, grid, x, y)
    for i = 1, grid.width do
      for j = 1, grid.height do
        this:set(x+i-1, y+j-1, grid:get(i, j))
      end
    end
  end
  return instance
end

-- WFC class
local wfc = {}
local output, colors, patterns, entropy, windowSize

function wfc:getColor(x, y)
  local id = output:get(x, y)
  if id then return colors[id] end
end

function wfc:init(image, outputWidth, outputHeight, windowSize_)
  windowSize = windowSize_
  local input = indexedImage(image.width, image.height)
  output = indexedImage(outputWidth, outputHeight)
  colors = {[0] = {r=0, g=0, b=0, a=0}}
  local colorIndices = {}
  -- replace colors with indices
  for x = 1, image.width do
    for y = 1, image.height do
      local pixel = image:get(x, y)
      local color = (pixel.r * 2^24) + (pixel.g * 2^16) + (pixel.b * 2^8) + pixel.a
      if not colorIndices[color] then
        colors[#colors + 1] = pixel
        colorIndices[color] = #colors
      end
      input:set(x, y, colorIndices[color])
    end
  end
  -- collect the patterns
  patterns = {}
  for x = 0, image.width - windowSize do
    for y = 0, image.height - windowSize do
        local pid = 0
        local pattern = grid(windowSize, windowSize)
        for i = 1, windowSize do
          for j = 1, windowSize do
            local index = input:get(x+i, y+j)
            pid = pid * #colors + index-1
            pattern:set(i, j, index)
          end
        end
        if not patterns[pid] then
          patterns[pid] = pattern
        end
        patterns[pid].count = patterns[pid].count or 0 + 1
    end
  end
  -- compute the correct entropy (it seems even incorrect ones work just for
  -- the purpose of generating an image)
  local sum, total = 0, 0
  for pid, pattern in pairs(patterns) do
    local count = pattern.count
    sum = sum + count*math.log(count)
    total = total + count
  end
  local e = math.log(total) - sum/total
  entropy = grid(outputWidth+1-windowSize, outputHeight+1-windowSize, e)
end

-- main loop of the WFC algorithm
function wfc:step()
  local x, y
  local min = 1/0
  local finished = true
  for i = 1, entropy.width do
    for j = 1, entropy.height do
      local e = entropy:get(i, j)
      -- add randomness to break ties
      if e ~= 0 then
        e = e + math.random(0, 10)/100
        if e < min then
          min, x, y = e, i, j
          finished = false
        end
      end
    end
  end
  if finished then return true end
  wfc:observe(x, y)
  wfc:propagate(x, y)
  return false
end

-- observe and collapse at a single coordinate
function wfc:observe(x, y)
  local matched = {}
  for i, pid in ipairs(wfc:match(x, y)) do
    local pattern = patterns[pid]
    for i = 1, pattern.count do -- insert the pattern ids with multiplicity
      matched[#matched + 1] = pid
    end
  end
  if #matched >= 1 then
    local index = math.random(#matched)
    output:blit(patterns[matched[index]], x, y)
  end
  entropy:set(x, y, 0)
end

-- propagate the entropy matrix, collapsing new matches in case
function wfc:propagate(x, y)
  for i = 0, windowSize-1 do
    for j = 0, windowSize-1 do
      local e = entropy:get(x+i, y+j) or 0
      if e > 0 then
        local matched = wfc:match(x+i, y+j)
        if #matched == 1 then
          local pid = matched[1]
          output:blit(patterns[pid], x+i, y+j)
          entropy:set(x+i, y+j, 0)
          -- further propagation could collapse a large portion of the image
          -- into a regular pattern, so we do this by a 1-in-5 chance
          if math.random(1, 5) == 1 then wfc:propagate(x+i, y+j) end
        elseif #matched > 1 then
          local sum, total = 0, 0
          for i, pid in ipairs(matched) do
            local count = patterns[pid].count
            sum = sum + count*math.log(count)
            total = total + count
          end
          entropy:set(x+i, y+j, math.log(total) - sum/total)
        end
      end
    end
  end
end

-- return the indices of the matched patterns
function wfc:match(x, y)
  local window = output:getWindow(x, y, windowSize)
  local matched = {}
  for pid, pattern in pairs(patterns) do
    local match = true
    for i = 1, windowSize do
      for j = 1, windowSize do
        local id = window:get(i, j)
        -- id is nil if out-of-bound; 0 if hasn't been determined
        if id and id > 0 and id ~= pattern:get(i, j) then
          match = false
          break
        end
      end
    end
    if match then
      matched[#matched + 1] = pid
    end
  end
  return matched
end

-------------------------------------------------------------------------------
-- The following is the main part of the user script
-------------------------------------------------------------------------------
OUTPUT.name = "wave function collapse"
local image = INPUT:getCurrent()
windowSize = MODIFIER.value or 3
local output = image.new(64, 64)
math.randomseed(os.time())
wfc:init(image, output.width, output.height, windowSize)
while not wfc:step() do end
for x = 1, output.width do
  for y = 1, output.height do
    local color = wfc:getColor(x, y)
    output:set(x, y, color.r, color.g, color.b, color.a)
  end
end
OUTPUT:changeCurrent(output)
