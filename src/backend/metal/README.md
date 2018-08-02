# gfx-backend-metal

[Metal](https://developer.apple.com/metal/) backend for gfx-rs.

## Details

### Normalized Coordinates

Render | Depth | Texture
-------|-------|--------
![render_coordinates](../../../info/gl_render_coordinates.png) | ![depth_coordinates](../../../info/dx_depth_coordinates.png) | ![texture_coordinates](../../../info/dx_texture_coordinates.png)

### Resource Mapping

Metal has flat resource spaces (bfufers, textures, samplers) per shader stage. Here is how we map Vulkan concepts to this space:
  1. [0 .. P) are the descriptor set resources, where P is the total of descriptors for this stage/type defined by the pipeline layout. Thus, changing the descriptor sets and pipelines within the same pipeline layout doesn't invalidate those bindings.
  2. [P .. P+V) are the vertex buffers, where V is the vertex buffer count (which we don't know in advance).
  3. (C) is the last binding available on the system, used for push constants that are provided as a data buffer. We check that P+V <= C to ensure no conflicts.
