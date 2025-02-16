-------------------------------------------------------------------------------
-- example of a user script
-------------------------------------------------------------------------------

-- compute an edge normal map from the alpha channel of the image
function computeEdgeNormal(image, radius)
  local normal = image:clone()
  -- loop over the pixels
  for x = 1, image.width do
    for y = 1, image.height do
      if image:get(x, y).a < 128 then
        -- we treat pixels with alpha < 128 as transparent
        normal:set(x, y, 0, 0, 0, 0)
      else
        local nx, ny, nz = 0, 0, radius
        -- do a weighted sum over all neighbors in the radius
        for dx = -radius, radius do
          for dy = -radius, radius do
            local neighbor = image:get(x + dx, y + dy)
            if not neighbor or neighbor.a < 128 then
              local norm = math.sqrt(dx*dx + dy*dy)
              nx = nx + dx/norm
              ny = ny + dy/norm
            end
          end
        end
        -- normalize the vector
        local norm = math.sqrt(nx*nx + ny*ny + nz*nz)
        local r, g, b = (nx/norm + 1)/2, (-ny/norm + 1)/2, (nz/norm + 1)/2
        normal:setLinear(x, y, r, g, b, 1, false)
      end
    end
  end
  return normal
end

-------------------------------------------------------------------------------
-- The following is the main part of the user script
-------------------------------------------------------------------------------
-- name of the script
OUTPUT.name = "compute edge normal"
-- we allow using modifier to change the radius of the computation
-- fallback to 1 if not present
radius = MODIFIER.value or 1
local currentLayer = INPUT.currentLayer
-- loop over all frames in the current layer
for frame = 1, INPUT.frameCount do
  local input = INPUT:get(currentLayer, frame)
  local output = computeEdgeNormal(input, radius)
  OUTPUT:change(currentLayer, frame, output)
end
