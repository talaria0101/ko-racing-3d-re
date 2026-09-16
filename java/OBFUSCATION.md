# Obfuscated -> readable name map

Generated files (`Obf*.java`, `Gen*.java`, `KORa.java`,
`VservManager.java`) come from `tools/jdeob.py` and must
not be hand edited. The curated race-path files
(e.g. `GameTrack.java` for `bs`) are the reviewed layer;
the matching `Gen*.java` file is the raw emitter output
kept beside it for review.

| original | file | role |
|---|---|---|
| `KORa` | `KORa.java` | <clinit>; <init>; Cancel; Code; java/util; javax/microedition |
| `VservManager` | `VservManager.java` |  Configuration/;  Profile/; %2D; %2F; java/io; java/util; javax/microedition |
| `a` | `GenObfA.java` | <init>; Code; [Li;; java/lang/Object |
| `aa` | `OrbitCam.java` | <init>; Code; abs; java/lang/Math; javax/microedition |
| `ab` | `Dialog.java` |  PTS; <init>; Code; [[B; javax/microedition |
| `ac` | `ObfAc.java` | <init>; Code; KORa; close; java/io; java/util |
| `ad` | `Countdown.java` | <init>; Code; LOST; getClipHeight; javax/microedition |
| `ae` | `MidList.java` | /lists/md_list; /tiles/; <init>; Code; java/io |
| `af` | `StringList.java` | <init>; Code; [Ljava/lang/String;; getClipWidth; javax/microedition |
| `ag` | `ImageGrid.java` | /images/i_s.png; <clinit>; <init>; Code; javax/microedition |
| `ah` | `ObfAh.java` | <init>; Code; java/io/IOException; StackMap |
| `ai` | `GenObfAi.java` | /models/; /tex/; <init>; Code |
| `aj` | `BackgroundPass.java` | <init>; Code; java/lang/Object; javax/microedition/m3g/Background; javax/microedition |
| `ak` | `ImageBackground.java` | <init>; Code; createImage; err; java/io; javax/microedition |
| `al` | `Settings.java` | /back/clear.bck; /back/desert.bck; /back/rain.bck; /back/snow.bck; java/io; java/util; javax/microed |
| `am` | `GenObfAm.java` | <init>; Code; java/lang/Object; javax/microedition/m3g/Appearance; javax/microedition |
| `an` | `FrameTimer.java` | <init>; Code; currentTimeMillis; java/lang/Object |
| `ao` | `Label.java` | <clinit>; <init>; Code; KORa; javax/microedition |
| `ap` | `HighList.java` | /lists/hd_list; /tiles/; <init>; Code; java/io |
| `aq` | `SysUtil.java` | .tab; /fonts/black; /fonts/font; /fonts/font.tab; java/io; javax/microedition; javax/wireless |
| `ar` | `GenObfAr.java` | /models/; /tex/; <clinit>; <init>; java/io |
| `as` | `EngineSounds.java` | /sounds/acc2_8k.amr; /sounds/acc3_8k.amr; /sounds/acc_8k.amr; /sounds/dec_8k.amr; java/util; javax/m |
| `at` | `GenObfAt.java` | /models/ps; /models/z; /tex/r.png; <clinit>; java/io; javax/microedition |
| `au` | `ObfAu.java` | <clinit>; <init>; Code; java/lang/Object |
| `av` | `AiTune.java` | <init>; Code; StackMap |
| `aw` | `PlayerTune.java` | <init>; Code; java/io/IOException; java/lang/Math |
| `ax` | `MenuItem.java` | <init>; Code; getClipHeight; getClipWidth; javax/microedition |
| `ay` | `Rain.java` | <init>; Code; [[F; [[I; java/util; javax/microedition |
| `az` | `ObfAz.java` | <init>; Code; StackMap; javax/microedition/lcdui/Graphics |
| `b` | `GenSceneryList.java` | /lists/ol; /objects/; <init>; Code; java/io |
| `ba` | `CarSpec.java` | /cars/; /models/; /tex/; <clinit>; java/io; javax/microedition |
| `bb` | `AssetManager.java` | <clinit>; <init>; Code; java/lang/Exception |
| `bc` | `GenObfBc.java` | /models/; /tex/; <init>; Code; java/io |
| `bd` | `GenObfBd.java` | /images/lg.png; /images/tc.png; /images/td.png; /images/tf.png; java/io; javax/microedition |
| `be` | `StreamReader.java` | <clinit>; <init>; Code; append; java/io |
| `bf` | `TileList.java` | /lists/tile_list; /tiles/; <init>; Code; java/io |
| `bg` | `ChaseNear.java` | <init>; Code; abs; java/lang/Math; javax/microedition |
| `bh` | `GenObfBh.java` | <init>; Code; [[I; abs; javax/microedition |
| `bi` | `MenuButton.java` | <init>; Code; KORa; buzz |
| `bj` | `Rect.java` | <init>; Code; java/lang/Object; StackMap |
| `bk` | `IconList.java` | <init>; Code; [Ldl;; [Ljava/lang/String;; javax/microedition |
| `bl` | `Resources.java` | .lvl; <clinit>; <init>; Code; java/io |
| `bm` | `GenObfBm.java` | <init>; Code; abs; java/io/PrintStream; java/io |
| `bn` | `Widget.java` | <init>; Code; java/lang/Object; StackMap |
| `bo` | `OpponentCar.java` | <init>; Code; abs; java/lang/Math |
| `bp` | `GenObfBp.java` | <init>; Code; java/lang/Exception; java/lang/Object |
| `bq` | `GenObfBq.java` | <init>; Code; bindTarget; clear; javax/microedition |
| `br` | `DeluxeTrackSelect.java` | /images/a.png; /images/map2.jpg; /images/qm2.png; /images/st.png; javax/microedition |
| `bs` | `GenObfBs.java` | <clinit>; <init>; Code; [[Lbm;; java/io; java/util; javax/microedition |
| `bt` | `GenObfBt.java` | /cars/; /images/snow; <clinit>; <init> |
| `bu` | `CareerScreen.java` | /levels/; <init>; Code; PLAYER; java/io; javax/microedition |
| `bv` | `RaceView.java` | <init>; Code; [Ljava/lang/String;; abs; java/io; javax/microedition |
| `bw` | `GridCell.java` | <init>; Code; StackMap |
| `bx` | `SoundBank.java` | <clinit>; <init>; Code; VolumeControl; java/io; java/util; javax/microedition |
| `by` | `Billboard.java` | <init>; Code; javax/microedition/m3g/Transform; postRotate; javax/microedition |
| `bz` | `Vec3.java` | <init>; Code; cos; java/lang/Math |
| `c` | `ChaseFar.java` | <init>; Code; abs; java/lang/Math; javax/microedition |
| `ca` | `ObfCa.java` | <init>; Code; java/lang/Object |
| `cb` | `ObfCb.java` | <init>; Code; drawRect; fillRect; javax/microedition |
| `cc` | `ObfCc.java` | <init>; Code; [Lbn;; drawLine; javax/microedition |
| `cd` | `ObfCd.java` | <clinit>; <init>; Code; KORa; java/io; java/util; javax/microedition |
| `ce` | `ObfCe.java` | <init>; Code; KORa; drawRect; java/util; javax/microedition |
| `cf` | `Textures.java` | /tex/r.png; <init>; Code; [Ljava/lang/String;; javax/microedition |
| `cg` | `ProgressBar.java` | <init>; Code; fillRect; getClipWidth; javax/microedition |
| `ch` | `ObfCh.java` | <init>; Code; currentTimeMillis; drawLine; javax/microedition |
| `ci` | `ImageCell.java` | <init>; Code; clipRect; drawImage; javax/microedition |
| `cj` | `BitWriter.java` | <init>; Code; flush; java/io/ByteArrayOutputStream; java/io |
| `ck` | `RaceLine.java` | <init>; Code; KORa; [Lz;; java/util |
| `cl` | `GenObfCl.java` | /models/; /tex/; /tex/shadow.png; <clinit>; java/io; java/util; javax/microedition |
| `cm` | `WidgetGroup.java` | <clinit>; <init>; Code; [Lbn;; javax/microedition |
| `cn` | `UiText.java` | /ui/bob.txt; <init>; Code; UTF-8; java/io |
| `co` | `Garage.java` | /images/add.png; /images/bob_full.png; /images/box.png; /images/bt.png; java/util; javax/microeditio |
| `cp` | `ObfCp.java` | <init>; Code; [Ljava/lang/String;; [Ljavax/microedition/lcdui/Image;; javax/microedition |
| `cq` | `Sprite.java` | <clinit>; <init>; Code; createImage; javax/microedition |
| `cr` | `CockpitCam.java` | <init>; Code; javax/microedition/m3g/Transform; postRotate; javax/microedition |
| `cs` | `StockTune.java` | <init>; Code |
| `ct` | `Plane.java` | <clinit>; <init>; Code; abs |
| `cu` | `MenuScreen.java` | <init>; Code; KORa; close; java/io; java/util; javax/microedition |
| `cv` | `MainCanvas.java` | /campaign/campaign; /campaign/deluxe; <clinit>; <init>; java/io; java/util; javax/microedition |
| `cw` | `Planet.java` | <clinit>; <init>; Code; KORa; java/util; javax/microedition |
| `cx` | `PlayerCar.java` | /models/; /tex/; /tex/shadow.png; <init> |
| `cy` | `AudioPlayer.java` | <clinit>; <init>; Code; VolumeControl; java/io; java/util; javax/microedition |
| `cz` | `ObfCz.java` | <init>; Code |
| `d` | `CarSelect.java` | <init>; Code; close; java/io/IOException; java/io |
| `da` | `BitReader.java` | <init>; Code; java/lang/Object; StackMap |
| `db` | `MenuList.java` | <init>; Code; KORa; [Lax;; javax/microedition |
| `dc` | `ObfDc.java` | <init>; Code; getClipHeight; getClipWidth; javax/microedition |
| `dd` | `ScoreRecord.java` | <init>; Code; append; getTime; java/io; java/util |
| `de` | `Meshes.java` | <init>; Code; [Lat;; [Ljava/lang/String;; javax/microedition |
| `df` | `Frustum.java` | <clinit>; <init>; Code; cos |
| `dg` | `SoundTrigger.java` | <init>; Code; abs; compareTo; javax/microedition |
| `dh` | `Bluetooth.java` | 32253635BF2E4FE0898AF505D430A394; ;authenticate=; ;encrypt=; ;master=; java/io; java/util; javax/blu |
| `di` | `CameraState.java` | <init>; Code; abs; java/lang/Math |
| `dj` | `ScoreUpload.java` | &p=; <init>; ?a=b; ?a=s&s=; java/io; javax/microedition |
| `dk` | `CareerMap.java` | 8a.map; <init>; Code; close; java/io; javax/microedition |
| `dl` | `ImageItem.java` | <init>; Code; drawImage; getClipHeight; javax/microedition |
| `dm` | `ColorBackground.java` | <init>; Code; javax/microedition/m3g/Background; setColor; javax/microedition |
| `dn` | `BitmapFont.java` | .png; <init>; Code; append; javax/microedition |
| `do` | `RaceTouchControls.java` | <init>; Code; getClipHeight; getClipWidth; javax/microedition |
| `e` | `TouchButtons.java` | /images/bp.png; /images/f.png; /images/l.png; /images/left.png; java/io; javax/microedition |
| `f` | `GlyphTable.java` | <init>; Code; UTF-8; [[B; java/io |
| `g` | `FontCodec.java` | <init>; Code; close; java/io/IOException; java/io |
| `h` | `ObfH.java` | <init>; Code; KORa; fillRect; java/util; javax/microedition |
| `i` | `Triangle.java` | <clinit>; <init>; Code; java/lang/Object |
| `j` | `GenObfJ.java` | <init>; Code; java/lang/Object; javax/microedition/m3g/Transform; javax/microedition |
| `k` | `ObfK.java` | <init>; Code; StackMap |
| `l` | `Snow.java` | .png; <clinit>; <init>; Code; java/util; javax/microedition |
| `m` | `MainMenu.java` |  PTS; /images/arr.png; /images/e.png; /images/fu.png; java/io; java/util; javax/microedition |
| `n` | `CheckBox.java` | /images/c.png; /images/r.png; <init>; Code; javax/microedition |
| `o` | `GhostCar.java` | <init>; Code; java/io/DataInputStream; java/io/IOException; java/io |
| `p` | `Font.java` | .png; <clinit>; <init>; Code; java/io; javax/microedition |
| `q` | `ObfQ.java` | <init>; Code; java/lang/Object |
| `r` | `GenObfR.java` | .car; /cars/; /images/lo.png; /images/los.png; java/io; java/util; javax/microedition |
| `s` | `MarqueeButton.java` | <clinit>; <init>; Code; getClipWidth; javax/microedition |
| `t` | `TextBuffer.java` | <init>; Code; UTF-8; [[B; java/io |
| `u` | `TrackSelect.java` | /images/a.png; /images/map.jpg; /images/qm.png; /images/st.png; java/io; javax/microedition |
| `v` | `RaceScreen.java` | <init>; ?a=m; Code; append; java/io; javax/microedition |
| `w` | `ObfW.java` | <clinit>; <init>; ?a=m; Code; java/io; javax/microedition |
| `x` | `CarPhysics.java` | <init>; Code; java/io/IOException; java/lang/Object |
| `y` | `Screen.java` | <init>; Code; KORa; buzz |
| `z` | `Vec2.java` | <init>; Code; append; java/lang/Math |
