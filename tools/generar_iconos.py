"""Genera los iconos de Aureon.

Marca: cuadrado redondeado grafito con una «A» dorada, coherente con el
símbolo de la barra lateral de la aplicación.
"""
import os
import struct

from PIL import Image, ImageDraw, ImageFont

RAIZ = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
ICONOS = os.path.join(RAIZ, "src-tauri", "icons")
FONTE = r"C:\Windows\Fonts\seguisb.ttf"

FONDO = (35, 33, 29)
FONDO_ALTO = (52, 49, 42)
DORADO = (201, 165, 95)
DORADO_ALTO = (232, 205, 148)


def base(tam: int) -> Image.Image:
    escala = tam / 1024
    img = Image.new("RGBA", (tam, tam), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)

    # Fondo con degradado vertical suave.
    for y in range(tam):
        t = y / max(tam - 1, 1)
        color = tuple(int(FONDO[i] + (FONDO_ALTO[i] - FONDO[i]) * t) for i in range(3))
        d.rectangle([0, y, tam - 1, y], fill=color + (255,))

    # Recorte redondeado.
    mascara = Image.new("L", (tam, tam), 0)
    ImageDraw.Draw(mascara).rounded_rectangle(
        [0, 0, tam - 1, tam - 1], radius=int(tam * 0.22), fill=255
    )
    img.putalpha(mascara)

    # Borde dorado fino.
    borde = Image.new("RGBA", (tam, tam), (0, 0, 0, 0))
    ImageDraw.Draw(borde).rounded_rectangle(
        [
            int(tam * 0.012),
            int(tam * 0.012),
            tam - 1 - int(tam * 0.012),
            tam - 1 - int(tam * 0.012),
        ],
        radius=int(tam * 0.21),
        outline=DORADO + (70,),
        width=max(1, int(tam * 0.012)),
    )
    img = Image.alpha_composite(img, borde)

    # Letra «A» en degradado dorado.
    fuente = ImageFont.truetype(FONTE, int(tam * 0.44))
    caja = fuente.getbbox("A")
    ancho = caja[2] - caja[0]
    alto = caja[3] - caja[1]
    x = (tam - ancho) / 2 - caja[0]
    y = (tam - alto) / 2 - caja[1]

    letra = Image.new("RGBA", (tam, tam), (0, 0, 0, 0))
    ImageDraw.Draw(letra).text((x, y), "A", font=fuente, fill=(255, 255, 255, 255))

    degradado = Image.new("RGBA", (tam, tam), (0, 0, 0, 0))
    dg = ImageDraw.Draw(degradado)
    for dy in range(tam):
        t = dy / max(tam - 1, 1)
        color = tuple(
            int(DORADO_ALTO[i] + (DORADO[i] - DORADO_ALTO[i]) * t) for i in range(3)
        )
        dg.rectangle([0, dy, tam - 1, dy], fill=color + (255,))
    degradado.putalpha(letra.getchannel("A"))
    img = Image.alpha_composite(img, degradado)
    return img


def escribir(tam: int, nombre: str) -> Image.Image:
    img = base(tam)
    img.save(os.path.join(ICONOS, nombre))
    return img


def escribir_ico() -> None:
    img = base(256)
    img.save(
        os.path.join(ICONOS, "icon.ico"),
        sizes=[(16, 16), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)],
    )


def escribir_icns(rutas: dict) -> None:
    """Construye un archivo ICNS con entradas PNG."""
    cuerpo = b""
    for tipo, ruta in rutas.items():
        with open(ruta, "rb") as f:
            datos = f.read()
        cuerpo += tipo.encode("ascii") + struct.pack(">I", len(datos) + 8) + datos
    with open(os.path.join(ICONOS, "icon.icns"), "wb") as f:
        f.write(b"icns" + struct.pack(">I", len(cuerpo) + 8) + cuerpo)


def main() -> None:
    os.makedirs(ICONOS, exist_ok=True)
    escribir(32, "32x32.png")
    escribir(128, "128x128.png")
    escribir(256, "128x128@2x.png")
    escribir(512, "icon.png")
    escribir(256, "icon-256.png")
    escribir(64, "icon-64.png")
    escribir_ico()

    for tam, nombre in [
        (44, "Square44x44Logo.png"),
        (71, "Square71x71Logo.png"),
        (72, "Square72x72Logo.png"),
        (75, "Square75x75Logo.png"),
        (142, "Square142x142Logo.png"),
        (150, "Square150x150Logo.png"),
        (152, "Square152x152Logo.png"),
        (284, "Square284x284Logo.png"),
        (310, "Square310x310Logo.png"),
        (50, "StoreLogo.png"),
    ]:
        escribir(tam, nombre)

    escribir_icns(
        {
            "ic11": os.path.join(ICONOS, "32x32.png"),
            "ic12": os.path.join(ICONOS, "icon-64.png"),
            "ic07": os.path.join(ICONOS, "128x128.png"),
            "ic08": os.path.join(ICONOS, "128x128@2x.png"),
            "ic09": os.path.join(ICONOS, "icon-256.png"),
            "ic10": os.path.join(ICONOS, "icon.png"),
        }
    )
    print("Iconos generados en", ICONOS)


if __name__ == "__main__":
    main()
