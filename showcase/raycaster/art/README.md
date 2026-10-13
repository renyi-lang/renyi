# The art of the showcase game

Taken from [Freedoom](https://github.com/freedoom/freedoom) at commit `3ac019628d70b812ffb722f7f2365771b76a9d29` by
`tools/freedoom_art.py`, which also writes `art.ry` beside this directory.
Freedoom is published under the 3-clause BSD license, kept here verbatim as
`COPYING.adoc`; the copyright is its authors'. The names are the lump names
Freedoom uses; a `Strip` or a `Picture` of `std.screen` names an image by its
file's stem, `play` a sound by its.

## walls

The wall textures, 64 or 128 columns by 128 rows; a strip takes a column modulo the width. The names in capitals are the textures of the original game that Freedoom builds from these patches.

| file | source | size | what |
|------|--------|------|------|
| `sw19_2.png` | `patches/sw19_2.png` | 64 by 128 | STARTAN3, the left half: grey panels on tan |
| `sw19_1.png` | `patches/sw19_1.png` | 64 by 128 | STARTAN3, the right half |
| `rw10_1.png` | `patches/rw10_1.png` | 64 by 128 | BRONZE1: riveted brown metal |
| `rw1_4.png` | `patches/rw1_4.png` | 64 by 128 | BRICK2: brown brick |
| `wall47_1.png` | `patches/wall47_1.png` | 64 by 128 | METAL: dark green metal |
| `wla128_1.png` | `patches/wla128_1.png` | 64 by 128 | SLADWALL: green panels |
| `wall62_1.png` | `patches/wall62_1.png` | 128 by 128 | BROWN96: brown metal, 128 wide |
| `mwall1_1.png` | `patches/mwall1_1.png` | 128 by 128 | MARBLE1: green marble, 128 wide |
| `w94_1.png` | `patches/w94_1.png` | 128 by 128 | TEKWALL4: circuitry, 128 wide |
| `door2_4.png` | `patches/door2_4.png` | 128 by 128 | BIGDOOR2: a grey door, 128 wide |
| `door9_2.png` | `patches/door9_2.png` | 128 by 128 | BIGDOOR3: a green door, 128 wide |

## sky

The sky, 256 columns by 128 rows, drawn stretched behind the walls.

| file | source | size | what |
|------|--------|------|------|
| `rsky1.png` | `patches/rsky1.png` | 256 by 128 | SKY1: brown clouds |

## demon

The demon: walking A to D, attacking E to G, hurt H, each in five rotations (1 facing the player, 2/8 turned 45 degrees, 3/7 side on, 4/6 turned 135 degrees, 5 from behind; 6 to 8 are 2 to 4 mirrored), dying I to N. The offsets are the anchor: the column of the sprite's middle and the row of its feet, from the top left.

| file | source | size | what |
|------|--------|------|------|
| `sarga1.png` | `sprites/sarga1.png` | 38 by 59, anchor 17, 55 | frame A, facing the player |
| `sarga2a8.png` | `sprites/sarga2a8.png` | 54 by 57 | frame A, turned 45 degrees |
| `sarga3a7.png` | `sprites/sarga3a7.png` | 58 by 58, anchor 29, 53 | frame A, side on |
| `sarga4a6.png` | `sprites/sarga4a6.png` | 47 by 57 | frame A, turned 135 degrees |
| `sarga5.png` | `sprites/sarga5.png` | 38 by 60 | frame A, from behind |
| `sargb1.png` | `sprites/sargb1.png` | 34 by 59 | frame B, facing the player |
| `sargb2b8.png` | `sprites/sargb2b8.png` | 49 by 58 | frame B, turned 45 degrees |
| `sargb3b7.png` | `sprites/sargb3b7.png` | 54 by 58 | frame B, side on |
| `sargb4b6.png` | `sprites/sargb4b6.png` | 46 by 57 | frame B, turned 135 degrees |
| `sargb5.png` | `sprites/sargb5.png` | 34 by 57 | frame B, from behind |
| `sargc1.png` | `sprites/sargc1.png` | 38 by 59, anchor 18, 55 | frame C, facing the player |
| `sargc2c8.png` | `sprites/sargc2c8.png` | 43 by 58 | frame C, turned 45 degrees |
| `sargc3c7.png` | `sprites/sargc3c7.png` | 54 by 57 | frame C, side on |
| `sargc4c6.png` | `sprites/sargc4c6.png` | 48 by 57 | frame C, turned 135 degrees |
| `sargc5.png` | `sprites/sargc5.png` | 38 by 60 | frame C, from behind |
| `sargd1.png` | `sprites/sargd1.png` | 38 by 59 | frame D, facing the player |
| `sargd2d8.png` | `sprites/sargd2d8.png` | 47 by 58 | frame D, turned 45 degrees |
| `sargd3d7.png` | `sprites/sargd3d7.png` | 54 by 57 | frame D, side on |
| `sargd4d6.png` | `sprites/sargd4d6.png` | 51 by 57 | frame D, turned 135 degrees |
| `sargd5.png` | `sprites/sargd5.png` | 34 by 57 | frame D, from behind |
| `sarge1.png` | `sprites/sarge1.png` | 38 by 60 | frame E, facing the player |
| `sarge2e8.png` | `sprites/sarge2e8.png` | 53 by 58 | frame E, turned 45 degrees |
| `sarge3e7.png` | `sprites/sarge3e7.png` | 56 by 58 | frame E, side on |
| `sarge4e6.png` | `sprites/sarge4e6.png` | 49 by 57 | frame E, turned 135 degrees |
| `sarge5.png` | `sprites/sarge5.png` | 34 by 57 | frame E, from behind |
| `sargf1.png` | `sprites/sargf1.png` | 38 by 60, anchor 18, 55 | frame F, facing the player |
| `sargf2f8.png` | `sprites/sargf2f8.png` | 52 by 57 | frame F, turned 45 degrees |
| `sargf3f7.png` | `sprites/sargf3f7.png` | 62 by 58 | frame F, side on |
| `sargf4f6.png` | `sprites/sargf4f6.png` | 51 by 57 | frame F, turned 135 degrees |
| `sargf5.png` | `sprites/sargf5.png` | 34 by 56 | frame F, from behind |
| `sargg1.png` | `sprites/sargg1.png` | 34 by 60 | frame G, facing the player |
| `sargg2g8.png` | `sprites/sargg2g8.png` | 46 by 58 | frame G, turned 45 degrees |
| `sargg3g7.png` | `sprites/sargg3g7.png` | 49 by 58 | frame G, side on |
| `sargg4g6.png` | `sprites/sargg4g6.png` | 43 by 57 | frame G, turned 135 degrees |
| `sargg5.png` | `sprites/sargg5.png` | 32 by 56 | frame G, from behind |
| `sargh1.png` | `sprites/sargh1.png` | 33 by 50, anchor 15, 47 | frame H, facing the player |
| `sargh2h8.png` | `sprites/sargh2h8.png` | 40 by 51 | frame H, turned 45 degrees |
| `sargh3h7.png` | `sprites/sargh3h7.png` | 43 by 52 | frame H, side on |
| `sargh4h6.png` | `sprites/sargh4h6.png` | 43 by 51 | frame H, turned 135 degrees |
| `sargh5.png` | `sprites/sargh5.png` | 33 by 51 | frame H, from behind |
| `sargi0.png` | `sprites/sargi0.png` | 38 by 59 | dying, frame I |
| `sargj0.png` | `sprites/sargj0.png` | 40 by 60 | dying, frame J |
| `sargk0.png` | `sprites/sargk0.png` | 43 by 53 | dying, frame K |
| `sargl0.png` | `sprites/sargl0.png` | 43 by 40 | dying, frame L |
| `sargm0.png` | `sprites/sargm0.png` | 48 by 30 | dying, frame M |
| `sargn0.png` | `sprites/sargn0.png` | 49 by 29 | dying, frame N |

## shotgun

The shotgun in the player's hands: still A, firing B to D; the muzzle flash A and B, drawn over it.

| file | source | size | what |
|------|--------|------|------|
| `shtga0.png` | `sprites/shtga0.png` | 67 by 62 | the shotgun, still |
| `shtgb0.png` | `sprites/shtgb0.png` | 88 by 128 | firing, frame B: pulled back |
| `shtgc0.png` | `sprites/shtgc0.png` | 94 by 143 | firing, frame C: pumped |
| `shtgd0.png` | `sprites/shtgd0.png` | 108 by 129 | firing, frame D: the pump returning |
| `shtfa0.png` | `sprites/shtfa0.png` | 36 by 47 | the muzzle flash, frame A |
| `shtfb0.png` | `sprites/shtfb0.png` | 58 by 55 | the muzzle flash, frame B |

## status bar

The status bar, 320 by 32, and what goes on it: the large red digits, the minus and the percent sign, the small yellow digits and the arms panel.

| file | source | size | what |
|------|--------|------|------|
| `stbar.png` | `graphics/stbar.png` | 320 by 32 | the bar |
| `starms.png` | `graphics/starms.png` | 38 by 32 | the arms panel |
| `sttnum0.png` | `graphics/sttnum0.png` | 13 by 16 | the large digit 0 |
| `sttnum1.png` | `graphics/sttnum1.png` | 13 by 16 | the large digit 1 |
| `sttnum2.png` | `graphics/sttnum2.png` | 13 by 16 | the large digit 2 |
| `sttnum3.png` | `graphics/sttnum3.png` | 13 by 16 | the large digit 3 |
| `sttnum4.png` | `graphics/sttnum4.png` | 13 by 16 | the large digit 4 |
| `sttnum5.png` | `graphics/sttnum5.png` | 13 by 16 | the large digit 5 |
| `sttnum6.png` | `graphics/sttnum6.png` | 13 by 16 | the large digit 6 |
| `sttnum7.png` | `graphics/sttnum7.png` | 13 by 16 | the large digit 7 |
| `sttnum8.png` | `graphics/sttnum8.png` | 13 by 16 | the large digit 8 |
| `sttnum9.png` | `graphics/sttnum9.png` | 13 by 16 | the large digit 9 |
| `sttminus.png` | `graphics/sttminus.png` | 13 by 16 | the large minus |
| `sttprcnt.png` | `graphics/sttprcnt.png` | 13 by 16 | the large percent sign |
| `stysnum0.png` | `graphics/stysnum0.png` | 4 by 6 | the small digit 0 |
| `stysnum1.png` | `graphics/stysnum1.png` | 4 by 6 | the small digit 1 |
| `stysnum2.png` | `graphics/stysnum2.png` | 4 by 6 | the small digit 2 |
| `stysnum3.png` | `graphics/stysnum3.png` | 4 by 6 | the small digit 3 |
| `stysnum4.png` | `graphics/stysnum4.png` | 4 by 6 | the small digit 4 |
| `stysnum5.png` | `graphics/stysnum5.png` | 4 by 6 | the small digit 5 |
| `stysnum6.png` | `graphics/stysnum6.png` | 4 by 6 | the small digit 6 |
| `stysnum7.png` | `graphics/stysnum7.png` | 4 by 6 | the small digit 7 |
| `stysnum8.png` | `graphics/stysnum8.png` | 4 by 6 | the small digit 8 |
| `stysnum9.png` | `graphics/stysnum9.png` | 4 by 6 | the small digit 9 |

## face

The player's face on the status bar, by health.

| file | source | size | what |
|------|--------|------|------|
| `stfst00.png` | `graphics/stfst00.png` | 24 by 29 | health level 0, look 0 |
| `stfst01.png` | `graphics/stfst01.png` | 24 by 29 | health level 0, look 1 |
| `stfst02.png` | `graphics/stfst02.png` | 24 by 29 | health level 0, look 2 |
| `stfst10.png` | `graphics/stfst10.png` | 24 by 29 | health level 1, look 0 |
| `stfst11.png` | `graphics/stfst11.png` | 24 by 29 | health level 1, look 1 |
| `stfst12.png` | `graphics/stfst12.png` | 24 by 29 | health level 1, look 2 |
| `stfst20.png` | `graphics/stfst20.png` | 24 by 30 | health level 2, look 0 |
| `stfst21.png` | `graphics/stfst21.png` | 24 by 30 | health level 2, look 1 |
| `stfst22.png` | `graphics/stfst22.png` | 24 by 30 | health level 2, look 2 |
| `stfst30.png` | `graphics/stfst30.png` | 24 by 31 | health level 3, look 0 |
| `stfst31.png` | `graphics/stfst31.png` | 24 by 31 | health level 3, look 1 |
| `stfst32.png` | `graphics/stfst32.png` | 24 by 31 | health level 3, look 2 |
| `stfst40.png` | `graphics/stfst40.png` | 24 by 31 | health level 4, look 0 |
| `stfst41.png` | `graphics/stfst41.png` | 24 by 31 | health level 4, look 1 |
| `stfst42.png` | `graphics/stfst42.png` | 24 by 31 | health level 4, look 2 |
| `stfouch0.png` | `graphics/stfouch0.png` | 24 by 31 | hurt, health level 0 |
| `stfouch1.png` | `graphics/stfouch1.png` | 24 by 31 | hurt, health level 1 |
| `stfouch2.png` | `graphics/stfouch2.png` | 24 by 31 | hurt, health level 2 |
| `stfouch3.png` | `graphics/stfouch3.png` | 24 by 31 | hurt, health level 3 |
| `stfouch4.png` | `graphics/stfouch4.png` | 24 by 31 | hurt, health level 4 |
| `stfevl0.png` | `graphics/stfevl0.png` | 24 by 30 | grinning, health level 0 |
| `stfevl1.png` | `graphics/stfevl1.png` | 24 by 30 | grinning, health level 1 |
| `stfevl2.png` | `graphics/stfevl2.png` | 24 by 30 | grinning, health level 2 |
| `stfevl3.png` | `graphics/stfevl3.png` | 24 by 30 | grinning, health level 3 |
| `stfevl4.png` | `graphics/stfevl4.png` | 24 by 30 | grinning, health level 4 |
| `stfkill0.png` | `graphics/stfkill0.png` | 24 by 30 | killed, health level 0 |
| `stfkill1.png` | `graphics/stfkill1.png` | 24 by 30 | killed, health level 1 |
| `stfkill2.png` | `graphics/stfkill2.png` | 24 by 30 | killed, health level 2 |
| `stfkill3.png` | `graphics/stfkill3.png` | 24 by 30 | killed, health level 3 |
| `stfkill4.png` | `graphics/stfkill4.png` | 24 by 30 | killed, health level 4 |
| `stfdead0.png` | `graphics/stfdead0.png` | 24 by 31 | dead |
| `stfgod0.png` | `graphics/stfgod0.png` | 24 by 29 | invulnerable |

## font

The large font, one file per character: STCFN followed by the ASCII code, 33 (!) to 95 (_), then 121, 123, 124 and 125.

| file | source | size | what |
|------|--------|------|------|
| `stcfn033.png` | `graphics/stcfn033.png` | 4 by 7 | the character '!' |
| `stcfn034.png` | `graphics/stcfn034.png` | 7 by 7 | the character '"' |
| `stcfn035.png` | `graphics/stcfn035.png` | 9 by 7 | the character '#' |
| `stcfn036.png` | `graphics/stcfn036.png` | 7 by 7 | the character '$' |
| `stcfn037.png` | `graphics/stcfn037.png` | 9 by 7 | the character '%' |
| `stcfn038.png` | `graphics/stcfn038.png` | 8 by 7 | the character '&' |
| `stcfn039.png` | `graphics/stcfn039.png` | 4 by 7 | the character "'" |
| `stcfn040.png` | `graphics/stcfn040.png` | 6 by 7 | the character '(' |
| `stcfn041.png` | `graphics/stcfn041.png` | 6 by 7 | the character ')' |
| `stcfn042.png` | `graphics/stcfn042.png` | 8 by 7 | the character '*' |
| `stcfn043.png` | `graphics/stcfn043.png` | 7 by 7 | the character '+' |
| `stcfn044.png` | `graphics/stcfn044.png` | 4 by 7 | the character ',' |
| `stcfn045.png` | `graphics/stcfn045.png` | 7 by 7 | the character '-' |
| `stcfn046.png` | `graphics/stcfn046.png` | 4 by 7 | the character '.' |
| `stcfn047.png` | `graphics/stcfn047.png` | 7 by 7 | the character '/' |
| `stcfn048.png` | `graphics/stcfn048.png` | 7 by 7 | the character '0' |
| `stcfn049.png` | `graphics/stcfn049.png` | 5 by 7 | the character '1' |
| `stcfn050.png` | `graphics/stcfn050.png` | 7 by 7 | the character '2' |
| `stcfn051.png` | `graphics/stcfn051.png` | 7 by 7 | the character '3' |
| `stcfn052.png` | `graphics/stcfn052.png` | 7 by 7 | the character '4' |
| `stcfn053.png` | `graphics/stcfn053.png` | 7 by 7 | the character '5' |
| `stcfn054.png` | `graphics/stcfn054.png` | 7 by 7 | the character '6' |
| `stcfn055.png` | `graphics/stcfn055.png` | 7 by 7 | the character '7' |
| `stcfn056.png` | `graphics/stcfn056.png` | 8 by 7 | the character '8' |
| `stcfn057.png` | `graphics/stcfn057.png` | 7 by 7 | the character '9' |
| `stcfn058.png` | `graphics/stcfn058.png` | 4 by 7 | the character ':' |
| `stcfn059.png` | `graphics/stcfn059.png` | 4 by 7 | the character ';' |
| `stcfn060.png` | `graphics/stcfn060.png` | 6 by 7 | the character '<' |
| `stcfn061.png` | `graphics/stcfn061.png` | 7 by 7 | the character '=' |
| `stcfn062.png` | `graphics/stcfn062.png` | 6 by 7 | the character '>' |
| `stcfn063.png` | `graphics/stcfn063.png` | 7 by 7 | the character '?' |
| `stcfn064.png` | `graphics/stcfn064.png` | 9 by 7 | the character '@' |
| `stcfn065.png` | `graphics/stcfn065.png` | 9 by 7 | the character 'A' |
| `stcfn066.png` | `graphics/stcfn066.png` | 8 by 7 | the character 'B' |
| `stcfn067.png` | `graphics/stcfn067.png` | 8 by 7 | the character 'C' |
| `stcfn068.png` | `graphics/stcfn068.png` | 8 by 7 | the character 'D' |
| `stcfn069.png` | `graphics/stcfn069.png` | 7 by 7 | the character 'E' |
| `stcfn070.png` | `graphics/stcfn070.png` | 7 by 7 | the character 'F' |
| `stcfn071.png` | `graphics/stcfn071.png` | 7 by 7 | the character 'G' |
| `stcfn072.png` | `graphics/stcfn072.png` | 7 by 7 | the character 'H' |
| `stcfn073.png` | `graphics/stcfn073.png` | 6 by 7 | the character 'I' |
| `stcfn074.png` | `graphics/stcfn074.png` | 7 by 7 | the character 'J' |
| `stcfn075.png` | `graphics/stcfn075.png` | 7 by 7 | the character 'K' |
| `stcfn076.png` | `graphics/stcfn076.png` | 7 by 7 | the character 'L' |
| `stcfn077.png` | `graphics/stcfn077.png` | 9 by 7 | the character 'M' |
| `stcfn078.png` | `graphics/stcfn078.png` | 8 by 7 | the character 'N' |
| `stcfn079.png` | `graphics/stcfn079.png` | 9 by 7 | the character 'O' |
| `stcfn080.png` | `graphics/stcfn080.png` | 8 by 7 | the character 'P' |
| `stcfn081.png` | `graphics/stcfn081.png` | 9 by 7 | the character 'Q' |
| `stcfn082.png` | `graphics/stcfn082.png` | 7 by 7 | the character 'R' |
| `stcfn083.png` | `graphics/stcfn083.png` | 8 by 7 | the character 'S' |
| `stcfn084.png` | `graphics/stcfn084.png` | 8 by 7 | the character 'T' |
| `stcfn085.png` | `graphics/stcfn085.png` | 8 by 7 | the character 'U' |
| `stcfn086.png` | `graphics/stcfn086.png` | 9 by 7 | the character 'V' |
| `stcfn087.png` | `graphics/stcfn087.png` | 9 by 7 | the character 'W' |
| `stcfn088.png` | `graphics/stcfn088.png` | 8 by 7 | the character 'X' |
| `stcfn089.png` | `graphics/stcfn089.png` | 8 by 7 | the character 'Y' |
| `stcfn090.png` | `graphics/stcfn090.png` | 7 by 7 | the character 'Z' |
| `stcfn091.png` | `graphics/stcfn091.png` | 6 by 7 | the character '[' |
| `stcfn092.png` | `graphics/stcfn092.png` | 7 by 7 | the character '\\' |
| `stcfn093.png` | `graphics/stcfn093.png` | 6 by 7 | the character ']' |
| `stcfn094.png` | `graphics/stcfn094.png` | 7 by 7 | the character '^' |
| `stcfn095.png` | `graphics/stcfn095.png` | 7 by 7 | the character '_' |
| `stcfn121.png` | `graphics/stcfn121.png` | 4 by 7 | the character 'y' |
| `stcfn123.png` | `graphics/stcfn123.png` | 7 by 7 | the character '{' |
| `stcfn124.png` | `graphics/stcfn124.png` | 4 by 7 | the character '|' |
| `stcfn125.png` | `graphics/stcfn125.png` | 7 by 7 | the character '}' |

## title

The title picture, 320 by 200.

| file | source | size | what |
|------|--------|------|------|
| `titlepic.png` | `graphics/titlepic/titlepic.png` | 320 by 200 | the title picture |

## sounds

The sounds, WAV files.

| file | source | size | what |
|------|--------|------|------|
| `dsshotgn.wav` | `sounds/dsshotgn.wav` | 11,235 bytes | the shotgun fired |
| `dssgcock.wav` | `sounds/dssgcock.wav` | 5,226 bytes | the shotgun pumped |
| `dssgtsit.wav` | `sounds/dssgtsit.wav` | 23,804 bytes | the demon sighting the player |
| `dssgtatk.wav` | `sounds/dssgtatk.wav` | 20,052 bytes | the demon biting |
| `dssgtdth.wav` | `sounds/dssgtdth.wav` | 32,674 bytes | the demon dying |
| `dsdmact.wav` | `sounds/dsdmact.wav` | 31,100 bytes | the demon roaming |
| `dsplpain.wav` | `sounds/dsplpain.wav` | 17,924 bytes | the player hurt |
| `dspldeth.wav` | `sounds/dspldeth.wav` | 8,317 bytes | the player dying |
| `dsdoropn.wav` | `sounds/dsdoropn.wav` | 36,490 bytes | a door opening |
| `dsdorcls.wav` | `sounds/dsdorcls.wav` | 36,770 bytes | a door closing |
| `dsnoway.wav` | `sounds/dsnoway.wav` | 8,574 bytes | a wall or a locked door bumped |
| `dsitemup.wav` | `sounds/dsitemup.wav` | 2,249 bytes | an item picked up |
