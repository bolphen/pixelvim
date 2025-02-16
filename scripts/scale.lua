-- scale the image to 2x by rounding corners
function scale(image)
  local output = image.new(image.width*2, image.height*2)
  for x = 1, image.width do
    for y = 1, image.height do
      local c  = image:get(x  , y  )
      local l  = image:get(x-1, y  )
      local r  = image:get(x+1, y  )
      local u  = image:get(x  , y-1)
      local d  = image:get(x  , y+1)
      local lu = image:get(x-1, y-1)
      local ru = image:get(x+1, y-1)
      local ld = image:get(x-1, y+1)
      local rd = image:get(x+1, y+1)
      if l and l == u and lu ~= c then
        output:set(2*x-1, 2*y-1, l.r, l.g, l.b, l.a)
      else
        output:set(2*x-1, 2*y-1, c.r, c.g, c.b, c.a)
      end
      if r and r == u and ru ~= c then
        output:set(2*x  , 2*y-1, r.r, r.g, r.b, r.a)
      else
        output:set(2*x  , 2*y-1, c.r, c.g, c.b, c.a)
      end
      if l and l == d and ld ~= c then
        output:set(2*x-1, 2*y  , l.r, l.g, l.b, l.a)
      else
        output:set(2*x-1, 2*y  , c.r, c.g, c.b, c.a)
      end
      if r and r == d and rd ~= c then
        output:set(2*x  , 2*y  , r.r, r.g, r.b, r.a)
      else
        output:set(2*x  , 2*y  , c.r, c.g, c.b, c.a)
      end
    end
  end
  return output
end

-------------------------------------------------------------------------------
-- The following is the main part of the user script
-------------------------------------------------------------------------------
OUTPUT.name = "scale2x"
for layer = 1, INPUT.layerCount do
  for frame = 1, INPUT.frameCount do
    local input = INPUT:get(layer, frame)
    local output = scale(input)
    OUTPUT:change(layer, frame, output)
  end
end
