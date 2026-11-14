<?xml version="1.0" encoding="UTF-8"?>
<tileset version="1.10" tiledversion="1.11.0" name="cave" tilewidth="16" tileheight="16" tilecount="32" columns="16">
 <properties>
  <property name="autotile" value="Tiles 0 to 15 are rock: the game picks the piece from the four neighbours (mask N 1, E 2, S 4, W 8)."/>
 </properties>
 <image source="cave.png" width="256" height="32"/>
 <tile id="0"><properties><property name="solid" type="bool" value="true"/></properties></tile>
 <tile id="1"><properties><property name="solid" type="bool" value="true"/></properties></tile>
 <tile id="2"><properties><property name="solid" type="bool" value="true"/></properties></tile>
 <tile id="3"><properties><property name="solid" type="bool" value="true"/></properties></tile>
 <tile id="4"><properties><property name="solid" type="bool" value="true"/></properties></tile>
 <tile id="5"><properties><property name="solid" type="bool" value="true"/></properties></tile>
 <tile id="6"><properties><property name="solid" type="bool" value="true"/></properties></tile>
 <tile id="7"><properties><property name="solid" type="bool" value="true"/></properties></tile>
 <tile id="8"><properties><property name="solid" type="bool" value="true"/></properties></tile>
 <tile id="9"><properties><property name="solid" type="bool" value="true"/></properties></tile>
 <tile id="10"><properties><property name="solid" type="bool" value="true"/></properties></tile>
 <tile id="11"><properties><property name="solid" type="bool" value="true"/></properties></tile>
 <tile id="12"><properties><property name="solid" type="bool" value="true"/></properties></tile>
 <tile id="13"><properties><property name="solid" type="bool" value="true"/></properties></tile>
 <tile id="14"><properties><property name="solid" type="bool" value="true"/></properties></tile>
 <tile id="15"><properties><property name="solid" type="bool" value="true"/></properties></tile>
 <tile id="18">
  <animation>
   <frame tileid="18" duration="450"/>
   <frame tileid="19" duration="450"/>
  </animation>
 </tile>
</tileset>
