<?xml version="1.0" encoding="UTF-8"?>
<tileset version="1.10" tiledversion="1.11.0" name="ground" tilewidth="16" tileheight="16" tilecount="8" columns="4">
 <image source="tiles.png" width="64" height="32"/>
 <tile id="1">
  <animation>
   <frame tileid="1" duration="200"/>
   <frame tileid="2" duration="300"/>
  </animation>
 </tile>
 <tile id="3" type="Wall">
  <properties>
   <property name="solid" type="bool" value="true"/>
  </properties>
 </tile>
 <tile id="5">
  <objectgroup draworder="index" id="2">
   <object id="1" x="0" y="8" width="16" height="8"/>
  </objectgroup>
 </tile>
 <tile id="6">
  <objectgroup draworder="index" id="2">
   <object id="1" x="0" y="16">
    <polygon points="0,0 16,0 16,-16"/>
   </object>
  </objectgroup>
 </tile>
 <tile id="7">
  <objectgroup draworder="index" id="2">
   <object id="1" x="0" y="0" width="16" height="16"/>
  </objectgroup>
 </tile>
</tileset>
