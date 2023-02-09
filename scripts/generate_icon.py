import svgwrite, os
from svgwrite import cm


def main():
    dir = os.path.dirname(__file__)
    draw(os.path.join(dir, f"F.svg"), "F", "#7743DB", "#FFFFFF")
    draw(os.path.join(dir, f"Q.svg"), "Q", "#7D8F69", "#FFFFFF")
    draw(os.path.join(dir, f"M.svg"), "M", "#F49D1A", "#FFFFFF")

def draw(filepath, letter, background_color, text_color):
    width = 1000
    dwg = svgwrite.Drawing(filename=filepath, size=(width, width), debug=True)
    font_size = 800
    dwg.add(dwg.rect(insert=(0,0), size=(width, width), rx=f"{width/6}", fill=background_color))
    dwg.add(dwg.text(f"{letter}", x=[width/2], y=[width/2+font_size/2-100], fill=text_color, stroke=text_color
        , text_anchor="middle", style=f"font: bold {font_size}px Arial;"))
    dwg.save()

if __name__ == "__main__":
    main()
