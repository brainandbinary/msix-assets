# msix-assets
Usage Examples
Default mode (contain – no distortion, adds transparent padding)
bash


```./msix-assets -s highres.png -t reference_icons/```


If your reference folder contains both square and wide images (e.g., StoreLogo.png 150x150, WideLogo.png 310x150), the tool will:

For square target → logo fits with equal padding.

For wide target → logo is scaled to fit height, then centered horizontally with transparent sides.

Cover mode (crops to fill the target)
bash


`./msix-assets -s highres.png -t reference_icons/ -m cover`


The logo will be scaled to completely cover the target area; parts outside the target are cropped away (center crop).

Stretch mode (distorts to exact size)
bash


`./msix-assets -s highres.png -t reference_icons/ -m stretch`


Force PNG output
bash


`./msix-assets -s highres.png -t reference_icons/ -f png`  

