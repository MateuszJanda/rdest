#!/bin/bash

VIDEO_FILE="caputerd_screen.mp4"

if [[ $1 == "-c" || $1 == "--capture" ]]; then
    ffmpeg -video_size 800x406 -framerate 25 -f x11grab -i :1.0+1,68 $VIDEO_FILE
elif [[ $1 == "-g" || $1 == "--video-to-gif" ]]; then
    PALETTE_FILE="tmp_pallete.png"
    OUTPUT_GIF="output.gif"
    FILTERS="fps=25"

    ffmpeg -v warning -i $VIDEO_FILE -vf "$FILTERS,palettegen" -y $PALETTE_FILE
    ffmpeg -v warning -i $VIDEO_FILE -i $PALETTE_FILE -lavfi "$FILTERS [x]; [x][1:v] paletteuse" -y $OUTPUT_GIF
else
    echo "Warning. No parameters."
    echo "Usage:"
    echo "   ./gif_maker.sh -c/--capture"
    echo "   ./gif_maker.sh -g/--video-to-gif"
fi
