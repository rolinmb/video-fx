package main

import (
    "image"
    "image/color"
    "math"
)

func DCT(block [][]float64) [][]float64 {
    n := len(block)
    dct := make([][]float64, n)
    for u := 0; u < n; u++ {
        dct[u] = make([]float64, n)
        for v := 0; v < n; v++ {
            var sum float64
            for i := 0; i < n; i++ {
                for j := 0; j < n; j++ {
                    cu := 1.0
                    cv := 1.0
                    if u == 0 {
                        cu = 1.0 / math.Sqrt2
                    }
                    if v == 0 {
                        cv = 1.0 / math.Sqrt2
                    }
                    sum += cu * cv * block[i][j] *
                        math.Cos((2*float64(i)+1)*float64(u)*math.Pi/(2*float64(n))) *
                        math.Cos((2*float64(j)+1)*float64(v)*math.Pi/(2*float64(n)))
                }
            }
            dct[u][v] = sum * (2.0 / math.Sqrt(float64(n)))
        }
    }

    return dct
}

func extractBlock(img *image.RGBA, x,y,blockSize int) [][]float64 {
    block := make([][]float64, blockSize)
    for i := 0; i < blockSize; i++ {
        block[i] = make([]float64, blockSize)
        for j := 0; j < blockSize; j++ {
            px := x + j
            py := y + i
            if px >= 0 && px < img.Bounds().Dx() && py >= 0 && py < img.Bounds().Dy() {
                r, _, _, _ := img.At(px, py).RGBA()
                gray := float64(r>>8) / 255.0
                block[i][j] = gray
            } else {
                block[i][j] = 0.0
            }
        }
    }
    return block
}

func storeBlock(img *image.RGBA, block [][]float64, x,y,blockSize int) {
    for i := 0; i < blockSize; i++ {
        for j := 0; j < blockSize; j++ {
            val := block[i][j]
            if val < 0 {
                val = 0
            } else if val > 1 {
                val = 1
            }
            gray := uint8(val * 255.0)
            img.Set(x+j, y+i, color.RGBA{R: gray, G: gray, B: gray, A: 255})
        }
    }
}

func applyDct(img *image.RGBA, blockSize int) *image.RGBA {
    bounds := img.Bounds()
    width, height := bounds.Dx(), bounds.Dy()
    dctImg := image.NewRGBA(bounds)
    for y := 0; y < height; y += blockSize {
        for x := 0; x < width; x += blockSize {
            block := extractBlock(img, x, y, blockSize)
            dctBlock := DCT(block)
            storeBlock(dctImg, dctBlock, x, y, blockSize)
        }
    }
    return dctImg
}

func DST(block [][]float64) [][]float64 {
    n := len(block)
    dst := make([][]float64, n)
    for u := 0; u < n; u++ {
        dst[u] = make([]float64, n)
        for v := 0; v < n; v++ {
            var sum float64
            for i := 0; i < n; i++ {
                for j := 0; j < n; j++ {
                    sum += block[i][j] *
                        math.Sin((float64(i)+0.5)*float64(u)*math.Pi/float64(n)) *
                        math.Sin((float64(j)+0.5)*float64(v)*math.Pi/float64(n))
                }
            }
            dst[u][v] = sum * (2.0 / math.Sqrt(float64(n)))
        }
    }
    return dst
}

func applyDst(img *image.RGBA, blockSize int) *image.RGBA {
    bounds := img.Bounds()
    width, height := bounds.Dx(), bounds.Dy()
    dstImg := image.NewRGBA(bounds)
    for y := 0; y < height; y += blockSize {
        for x := 0; x < width; x += blockSize {
            block := extractBlock(img, x, y, blockSize)
            dstBlock := DST(block)
            storeBlock(dstImg, dstBlock, x, y, blockSize)
        }
    }
    return dstImg
}