# Nine Slice

An original FigCraft tool plugin (`org.figcraft.tools.nine-slice`) generating resizable 9-slice UI panels from image rectangles.

## Usage

1. Select an image rectangle.
2. Run **Plugins ▸ Nine Slice ▸ Nine Slice…** to configure insets.
3. Insets default to `0` (automatic 1/3 division). Entering positive layer-unit values sets custom border dimensions.
4. Click **Apply** to generate a Frame with 9 child rectangles with proper constraint mappings.
5. The resulting frame cannot be resized below its border inset dimensions.
