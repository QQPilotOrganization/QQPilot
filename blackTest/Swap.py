from PIL import Image
from typing import Tuple,List,Any
import math
darkMain=Image.open("darkMain.png")
defaultMain=Image.open("defaultMain.png")
def scale(image:Image.Image,factor:float):
    w2=math.floor(image.width*factor)
    h2=math.floor(image.height*factor)
    return image.resize((w2,h2))
def colorStatistic(image:Image.Image):
    image=image.convert("RGBA")
    result=dict()
    for y in range(image.height):
        for x in range(image.width):
            px=image.getpixel((x,y))
            if px not in result:
                result[px]=1
            else:
                result[px]+=1
    return result
def getMax(d:dict)->Tuple[Tuple[int,int,int,int],int]:
    max_key = max(d, key=d.get)
    max_value = d[max_key]
    return max_key,max_value
def colorSwap(image:Image.Image,args:List[Tuple[Any,Any]]):
    image=image.convert("RGBA")
    image2=image.copy()
    for y in range(image.height):
        for x in range(image.width):
            px=image.getpixel((x,y))
            for arg in args:
                color1=arg[0]
                color2=arg[1]
                if px==color1:
                    image2.putpixel((x,y),color2)
                elif px==color2:
                    image2.putpixel((x,y),color1)
    return image2

darkChat=scale(Image.open("darkChat.png"),0.1)
cs=colorStatistic(darkChat)
print(getMax(cs)[0])
# print("-------------------")
# print(getMax(cs2)[0])

# swappedMain=colorSwap(darkMain,[((245, 245, 245, 255),(34, 34, 34, 255)),((47,47,47, 255),(235,235,235, 255))])
# swappedMain.save("swapped.png")

colorSwap(darkChat,[((245, 245, 245, 255),(34, 34, 34, 255)),((47,47,47, 255),(235,235,235, 255)),((*([27]*3), 255),(*([255]*3), 255))]).save("swappedChat.png")