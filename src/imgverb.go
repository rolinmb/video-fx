package main

import (
    "image"
    "image/color"
    "image/draw"
)

func imageReverb(imgSrc image.Image, sampleRate,reverbLenMs,decayFactor,damping float64) *image.RGBA {
    srcBounds := imgSrc.Bounds()
    newRgba := image.NewRGBA(srcBounds)
    draw.Draw(newRgba, srcBounds, imgSrc, srcBounds.Min, draw.Src)
    width := srcBounds.Max.X
    height := srcBounds.Max.Y
    processedSamples := make([][]color.RGBA, height)
    reverbLen := int((reverbLenMs / 1000.0) * sampleRate)
    for y := range processedSamples {
        processedSamples[y] = make([]color.RGBA, width)
        for x := range processedSamples[y] {
            processedSamples[y][x] = color.RGBAModel.Convert(imgSrc.At(x, y)).(color.RGBA)
        }
    }
    for y := 0; y < height; y++ {
        for x := 0; x < width; x++ {
            srcColor := color.RGBAModel.Convert(imgSrc.At(x, y)).(color.RGBA)
            if (x + reverbLen) < width {
                reverbColor := processedSamples[y][x + reverbLen]
                reverbColor.R = uint8(float64(reverbColor.R)*decayFactor + float64(srcColor.R)*(1.0-decayFactor))
				reverbColor.G = uint8(float64(reverbColor.G)*decayFactor + float64(srcColor.G)*(1.0-decayFactor))
				reverbColor.B = uint8(float64(reverbColor.B)*decayFactor + float64(srcColor.B)*(1.0-decayFactor))
				reverbColor.A = uint8(float64(reverbColor.A)*decayFactor + float64(srcColor.A)*(1.0-decayFactor))
                if x > 0 {
                    prevColor := processedSamples[y][x + reverbLen - 1]
					reverbColor.R = uint8(float64(reverbColor.R)*(1.0-damping) + float64(prevColor.R)*damping)
					reverbColor.G = uint8(float64(reverbColor.G)*(1.0-damping) + float64(prevColor.G)*damping)
					reverbColor.B = uint8(float64(reverbColor.B)*(1.0-damping) + float64(prevColor.B)*damping)
					reverbColor.A = uint8(float64(reverbColor.A)*(1.0-damping) + float64(prevColor.A)*damping)
                }
                processedSamples[y][x + reverbLen] = reverbColor
            }
            newRgba.Set(x, y, srcColor)
        }
    }
    for y := range processedSamples {
        for x := range processedSamples[y] {
            newRgba.Set(x, y, processedSamples[y][x])
        }
    }
    return newRgba
}