# Image to Tetris

This project aims to be able to approximate arbitrary images using valid board configurations for Tetris. This project follows 
a few rules in order to attempt to be faithful to the Tetris spirit:

1. The image is separated into a grid of `w x h` minos (or grid cells), and this board will only be filled up with valid configurations of tetrominos.
    * Garbage clears will not be factored in since garbage clears allow for grids to be colored arbitrarily without regard for piece shape. For example, [this player](https://www.youtube.com/watch?v=sSZA_W1hj08) was able to draw an image that no longer contains most of the original piece shapes by using smart piece placements and clears. I want to see piece shapes in the image approximations, so garbage clears will not be allowed
        * This also means there will be no garbage well in the image approximation.
    * Gaps caused by tetrominos will be allowed to be filled up by garbage minos. It is true that garbage tends to only appear by row and not by mino, but this was a compromise to allow the entire board to be filled with minos.
2. Multiple skins of tetris blocks will be allowed in any given image to approximate images more easily. This functionality can be tuned 
by adding or removing the skins in `./assets` so that this program can change which skins it has available.
3. Each skin has 9 minos of different colors/designs such that they correspond to the 9 common types of minos (hurry-up garbage, regular garbage, Z, L, O, S, I, J, T), and these mino designs will only be used with the correct piece shape.

```sh
# Get started approximating an image
> cargo run --release -- approx-image source.png output.png 32 32

# Example approximating a video
> cargo run --release -- approx-video source.mp4 output.mp4 32 32

```

## Requirements

The skins used for this application come from the [Jstris Customization Database](https://docs.google.com/spreadsheets/d/1xO8DTORacMmSJAQicpJscob7WUkOVuaNH0wzkR_X194/htmlview). **IMPORTANT**: that the rights to the skins are not owned by me. Once you have chosen the skins you want to use, create the directory `./assets` and place the skins' files there. At runtime, `image-to-tetris` will pick blocks from the skins assorted there.

Integration testing will source test images from the `./sources` directory. To test properly, have at least 1 image there and do not mix non-image files inside.

The `approx_video` functionality requires `ffmpeg`'s cli functionality, and it also uses `ffmpeg-next` for video processing 
reasons.

## Options

### approx-image
```
Usage: image-to-tetris approx-image <SOURCE> <OUTPUT> <BOARD_WIDTH> <BOARD_HEIGHT>

Arguments:
  <SOURCE>
  <OUTPUT>
  <BOARD_WIDTH>
  <BOARD_HEIGHT>
```

### approx-video
```
Usage: image-to-tetris approx-video <SOURCE> <OUTPUT> <BOARD_WIDTH> <BOARD_HEIGHT>

Arguments:
  <SOURCE>
  <OUTPUT>
  <BOARD_WIDTH>
  <BOARD_HEIGHT>
```

### Other Options
```
  -t, --threads <THREADS>      number of threads to use; default is 4
  -p, --prioritize-tetrominos  flag for whether to prioritize tetrominos or not; increases image color but reduces accuracy
  -h, --help                   Print help
  -V, --version                Print version
```

