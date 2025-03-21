function outline(image, color)
  local output = image:clone()
  local bg = image:get(1, 1)
  for x = 1, image.width do
    for y = 1, image.height do
      local c  = image:get(x, y)
      if c == bg then
        local l  = image:get(x-1, y  )
        local r  = image:get(x+1, y  )
        local u  = image:get(x  , y-1)
        local d  = image:get(x  , y+1)
        if l and l ~= bg or r and r ~= bg or
           u and u ~= bg or d and d ~= bg then
          output:set(x, y, color.r, color.g, color.b, color.a)
        end
      end
    end
  end
  return output
end

-------------------------------------------------------------------------------
-- The following is the main part of the user script
-------------------------------------------------------------------------------
OUTPUT.name = "outline"
for layer = 1, INPUT.layerCount do
  for frame = 1, INPUT.frameCount do
    local input = INPUT:get(layer, frame)
    local output = outline(input, COLOR)
    OUTPUT:change(layer, frame, output)
  end
end
