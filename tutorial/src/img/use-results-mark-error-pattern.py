import matplotlib.pyplot as plt
from matplotlib.path import Path
from matplotlib.patches import Rectangle
import os
from PIL import Image
def wrap_path(filename):
    return os.path.join(os.path.dirname(os.path.abspath(__file__)), filename)

FIGURE_WIDTH = 30
INDEX_TEXT_FONT_SIZE = 60
TEXT_FONT_SIZE = 100
COLOR_TEXT_ERR = "red"
COLOR_TEXT = "black"
FONTWEIGHT = "bold"
FONTWEIGHT_ROUND = "normal"


def main():
    # draw_error_3D(debug=False)
    pass  # the output file is too large, remember to compress it

def draw_error_3D(debug=False):
    fig, ax = no_axes_figure("use-results")
    img = Image.open(wrap_path("use-results.png"))
    width, height = img.size
    img = img.crop((width / 2 - height / 2, 0, width / 2 + height / 2, height))
    ax.imshow(img)
    for i in range(-1, 13):
        for pos in [(195 + 90 * i, 40, "j"), (30, 205 + 90 * i, "i"), (195 + 90 * i, height - 40, "j"), (width - 50, 205 + 90 * i, "i")]:
            ax.text(pos[0], pos[1], f"{i if i >= 0 else pos[2]}", fontsize=INDEX_TEXT_FONT_SIZE if i >= 0 else 70, color=COLOR_TEXT, fontweight=FONTWEIGHT
                , horizontalalignment='center', verticalalignment='center')
        
    error_pattern = {
        (0, 2): 'Z', (2, 4): 'Z', (3, 9): 'Z', (5, 1): 'Z', (6, 2): 'Z', (10, 4): 'Z'
        , (10, 8): 'Y', (5, 3): 'X', (7, 3): 'X', (12, 8): 'X', (7, 9): 'X', (5, 11): 'X'
    }
    for pos in error_pattern:
        error_type = error_pattern[pos]
        cx = 195 + 90 * pos[1]
        cy = 205 + 90 * pos[0]
        width = 80
        ax.add_patch(Rectangle((cx - width/2, cy - width/2 - 5), width, width, color="blue"))
        ax.text(cx, cy, f"{error_type}", fontsize=TEXT_FONT_SIZE, color=COLOR_TEXT_ERR, fontweight=FONTWEIGHT
            , horizontalalignment='center', verticalalignment='center')
    if debug:
        plt.show()
    fig.savefig(wrap_path('use-results-marked-uncompressed.png'))
    plt.close()

def no_axes_figure(name, size=(FIGURE_WIDTH, FIGURE_WIDTH)):
    fig = plt.figure(name)
    fig.set_size_inches(size)
    ax = plt.Axes(fig, [0,0,1,1])
    ax.set_axis_off()
    fig.add_axes(ax)
    return fig, ax

if __name__ == "__main__":
    main()
