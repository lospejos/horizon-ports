//! Written by tools/spelunkery_port.py: the vanilla loot tables the jar's dynamic pack (ModLootOverrides)
//! replaced with the live server's config, in 26.3's form; lib.rs swaps them in. Don't edit by hand.

pub(crate) const JSON: &str = r##"{
 "minecraft:blocks/deepslate_coal_ore": {
  "pools": [
   {
    "bonus_rolls": 0,
    "entries": [
     {
      "children": [
       {
        "condition": "minecraft:tool/can_silk_touch",
        "type": "minecraft:empty"
       },
       {
        "modifier": [
         {
          "add": false,
          "count": {
           "max": 6,
           "min": 3,
           "type": "minecraft:uniform"
          },
          "type": "minecraft:set_count"
         },
         {
          "enchantment": "minecraft:fortune",
          "formula": "minecraft:ore_drops",
          "type": "minecraft:apply_bonus"
         },
         {
          "type": "minecraft:explosion_decay"
         }
        ],
        "name": "spelunkery:coal_lump",
        "type": "minecraft:item"
       }
      ],
      "type": "minecraft:alternatives"
     }
    ],
    "rolls": 1
   },
   {
    "bonus_rolls": 0,
    "entries": [
     {
      "children": [
       {
        "condition": "minecraft:tool/can_silk_touch",
        "name": "minecraft:deepslate_coal_ore",
        "type": "minecraft:item"
       },
       {
        "modifier": [
         {
          "enchantment": "minecraft:fortune",
          "formula": "minecraft:ore_drops",
          "type": "minecraft:apply_bonus"
         },
         {
          "type": "minecraft:explosion_decay"
         }
        ],
        "name": "minecraft:coal",
        "type": "minecraft:item"
       }
      ],
      "type": "minecraft:alternatives"
     }
    ],
    "rolls": 1
   }
  ],
  "random_sequence": "minecraft:blocks/deepslate_coal_ore",
  "type": "minecraft:block"
 },
 "minecraft:blocks/deepslate_copper_ore": {
  "pools": [
   {
    "bonus_rolls": 0,
    "entries": [
     {
      "children": [
       {
        "condition": "minecraft:tool/can_silk_touch",
        "type": "minecraft:empty"
       },
       {
        "modifier": [
         {
          "add": false,
          "count": {
           "max": 6,
           "min": 3,
           "type": "minecraft:uniform"
          },
          "type": "minecraft:set_count"
         },
         {
          "enchantment": "minecraft:fortune",
          "formula": "minecraft:ore_drops",
          "type": "minecraft:apply_bonus"
         },
         {
          "type": "minecraft:explosion_decay"
         }
        ],
        "name": "spelunkery:raw_copper_nugget",
        "type": "minecraft:item"
       }
      ],
      "type": "minecraft:alternatives"
     }
    ],
    "rolls": 1
   },
   {
    "bonus_rolls": 0,
    "entries": [
     {
      "children": [
       {
        "condition": "minecraft:tool/can_silk_touch",
        "name": "minecraft:deepslate_copper_ore",
        "type": "minecraft:item"
       },
       {
        "modifier": [
         {
          "add": false,
          "count": {
           "max": 5,
           "min": 2,
           "type": "minecraft:uniform"
          },
          "type": "minecraft:set_count"
         },
         {
          "enchantment": "minecraft:fortune",
          "formula": "minecraft:ore_drops",
          "type": "minecraft:apply_bonus"
         },
         {
          "type": "minecraft:explosion_decay"
         }
        ],
        "name": "minecraft:raw_copper",
        "type": "minecraft:item"
       }
      ],
      "type": "minecraft:alternatives"
     }
    ],
    "rolls": 1
   }
  ],
  "random_sequence": "minecraft:blocks/deepslate_copper_ore",
  "type": "minecraft:block"
 },
 "minecraft:blocks/deepslate_diamond_ore": {
  "pools": [
   {
    "bonus_rolls": 0,
    "entries": [
     {
      "children": [
       {
        "condition": "minecraft:tool/can_silk_touch",
        "name": "minecraft:deepslate_diamond_ore",
        "type": "minecraft:item"
       },
       {
        "modifier": [
         {
          "enchantment": "minecraft:fortune",
          "formula": "minecraft:ore_drops",
          "type": "minecraft:apply_bonus"
         },
         {
          "type": "minecraft:explosion_decay"
         }
        ],
        "name": "spelunkery:rough_diamond",
        "type": "minecraft:item"
       }
      ],
      "type": "minecraft:alternatives"
     }
    ],
    "rolls": 1
   },
   {
    "bonus_rolls": 0,
    "entries": [
     {
      "children": [
       {
        "condition": "minecraft:tool/can_silk_touch",
        "type": "minecraft:empty"
       },
       {
        "modifier": [
         {
          "add": false,
          "count": {
           "max": 2,
           "min": 0,
           "type": "minecraft:uniform"
          },
          "type": "minecraft:set_count"
         },
         {
          "enchantment": "minecraft:fortune",
          "formula": "minecraft:ore_drops",
          "type": "minecraft:apply_bonus"
         },
         {
          "type": "minecraft:explosion_decay"
         }
        ],
        "name": "spelunkery:rough_diamond_shard",
        "type": "minecraft:item"
       }
      ],
      "type": "minecraft:alternatives"
     }
    ],
    "rolls": 1
   }
  ],
  "random_sequence": "minecraft:blocks/deepslate_diamond_ore",
  "type": "minecraft:block"
 },
 "minecraft:blocks/deepslate_emerald_ore": {
  "pools": [
   {
    "bonus_rolls": 0,
    "entries": [
     {
      "children": [
       {
        "condition": "minecraft:tool/can_silk_touch",
        "name": "minecraft:deepslate_emerald_ore",
        "type": "minecraft:item"
       },
       {
        "modifier": [
         {
          "enchantment": "minecraft:fortune",
          "formula": "minecraft:ore_drops",
          "type": "minecraft:apply_bonus"
         },
         {
          "type": "minecraft:explosion_decay"
         }
        ],
        "name": "spelunkery:rough_emerald",
        "type": "minecraft:item"
       }
      ],
      "type": "minecraft:alternatives"
     }
    ],
    "rolls": 1
   },
   {
    "bonus_rolls": 0,
    "entries": [
     {
      "children": [
       {
        "condition": "minecraft:tool/can_silk_touch",
        "type": "minecraft:empty"
       },
       {
        "modifier": [
         {
          "add": false,
          "count": {
           "max": 2,
           "min": 0,
           "type": "minecraft:uniform"
          },
          "type": "minecraft:set_count"
         },
         {
          "enchantment": "minecraft:fortune",
          "formula": "minecraft:ore_drops",
          "type": "minecraft:apply_bonus"
         },
         {
          "type": "minecraft:explosion_decay"
         }
        ],
        "name": "spelunkery:rough_emerald_shard",
        "type": "minecraft:item"
       }
      ],
      "type": "minecraft:alternatives"
     }
    ],
    "rolls": 1
   }
  ],
  "random_sequence": "minecraft:blocks/deepslate_emerald_ore",
  "type": "minecraft:block"
 },
 "minecraft:blocks/deepslate_gold_ore": {
  "pools": [
   {
    "bonus_rolls": 0,
    "entries": [
     {
      "children": [
       {
        "condition": "minecraft:tool/can_silk_touch",
        "type": "minecraft:empty"
       },
       {
        "modifier": [
         {
          "add": false,
          "count": {
           "max": 4,
           "min": 1,
           "type": "minecraft:uniform"
          },
          "type": "minecraft:set_count"
         },
         {
          "enchantment": "minecraft:fortune",
          "formula": "minecraft:ore_drops",
          "type": "minecraft:apply_bonus"
         },
         {
          "type": "minecraft:explosion_decay"
         }
        ],
        "name": "spelunkery:raw_gold_nugget",
        "type": "minecraft:item"
       }
      ],
      "type": "minecraft:alternatives"
     }
    ],
    "rolls": 1
   },
   {
    "bonus_rolls": 0,
    "entries": [
     {
      "children": [
       {
        "condition": "minecraft:tool/can_silk_touch",
        "name": "minecraft:deepslate_gold_ore",
        "type": "minecraft:item"
       },
       {
        "modifier": [
         {
          "enchantment": "minecraft:fortune",
          "formula": "minecraft:ore_drops",
          "type": "minecraft:apply_bonus"
         },
         {
          "type": "minecraft:explosion_decay"
         }
        ],
        "name": "minecraft:raw_gold",
        "type": "minecraft:item"
       }
      ],
      "type": "minecraft:alternatives"
     }
    ],
    "rolls": 1
   }
  ],
  "random_sequence": "minecraft:blocks/deepslate_gold_ore",
  "type": "minecraft:block"
 },
 "minecraft:blocks/deepslate_iron_ore": {
  "pools": [
   {
    "bonus_rolls": 0,
    "entries": [
     {
      "children": [
       {
        "condition": "minecraft:tool/can_silk_touch",
        "type": "minecraft:empty"
       },
       {
        "children": [
         {
          "condition": {
           "chance": 0.125,
           "type": "minecraft:random_chance"
          },
          "modifier": [
           {
            "add": false,
            "count": {
             "max": 2,
             "min": 1,
             "type": "minecraft:uniform"
            },
            "type": "minecraft:set_count"
           },
           {
            "enchantment": "minecraft:fortune",
            "formula": "minecraft:ore_drops",
            "type": "minecraft:apply_bonus"
           },
           {
            "type": "minecraft:explosion_decay"
           }
          ],
          "name": "spelunkery:raw_magnetite_nugget",
          "type": "minecraft:item"
         },
         {
          "modifier": [
           {
            "add": false,
            "count": {
             "max": 4,
             "min": 1,
             "type": "minecraft:uniform"
            },
            "type": "minecraft:set_count"
           },
           {
            "enchantment": "minecraft:fortune",
            "formula": "minecraft:ore_drops",
            "type": "minecraft:apply_bonus"
           },
           {
            "type": "minecraft:explosion_decay"
           }
          ],
          "name": "spelunkery:raw_iron_nugget",
          "type": "minecraft:item"
         }
        ],
        "type": "minecraft:alternatives"
       }
      ],
      "type": "minecraft:alternatives"
     }
    ],
    "rolls": 1
   },
   {
    "bonus_rolls": 0,
    "entries": [
     {
      "children": [
       {
        "condition": "minecraft:tool/can_silk_touch",
        "name": "minecraft:deepslate_iron_ore",
        "type": "minecraft:item"
       },
       {
        "modifier": [
         {
          "enchantment": "minecraft:fortune",
          "formula": "minecraft:ore_drops",
          "type": "minecraft:apply_bonus"
         },
         {
          "type": "minecraft:explosion_decay"
         }
        ],
        "name": "minecraft:raw_iron",
        "type": "minecraft:item"
       }
      ],
      "type": "minecraft:alternatives"
     }
    ],
    "rolls": 1
   }
  ],
  "random_sequence": "minecraft:blocks/deepslate_iron_ore",
  "type": "minecraft:block"
 },
 "minecraft:blocks/deepslate_lapis_ore": {
  "pools": [
   {
    "bonus_rolls": 0,
    "entries": [
     {
      "children": [
       {
        "condition": "minecraft:tool/can_silk_touch",
        "name": "minecraft:deepslate_lapis_ore",
        "type": "minecraft:item"
       },
       {
        "modifier": [
         {
          "add": false,
          "count": {
           "max": 2,
           "min": 1,
           "type": "minecraft:uniform"
          },
          "type": "minecraft:set_count"
         },
         {
          "enchantment": "minecraft:fortune",
          "formula": "minecraft:ore_drops",
          "type": "minecraft:apply_bonus"
         },
         {
          "type": "minecraft:explosion_decay"
         }
        ],
        "name": "spelunkery:rough_lazurite",
        "type": "minecraft:item"
       }
      ],
      "type": "minecraft:alternatives"
     }
    ],
    "rolls": 1
   },
   {
    "bonus_rolls": 0,
    "entries": [
     {
      "children": [
       {
        "condition": "minecraft:tool/can_silk_touch",
        "type": "minecraft:empty"
       },
       {
        "modifier": [
         {
          "add": false,
          "count": {
           "max": 7,
           "min": 2,
           "type": "minecraft:uniform"
          },
          "type": "minecraft:set_count"
         },
         {
          "enchantment": "minecraft:fortune",
          "formula": "minecraft:ore_drops",
          "type": "minecraft:apply_bonus"
         },
         {
          "type": "minecraft:explosion_decay"
         }
        ],
        "name": "spelunkery:rough_lazurite_shard",
        "type": "minecraft:item"
       }
      ],
      "type": "minecraft:alternatives"
     }
    ],
    "rolls": 1
   }
  ],
  "random_sequence": "minecraft:blocks/deepslate_lapis_ore",
  "type": "minecraft:block"
 },
 "minecraft:blocks/deepslate_redstone_ore": {
  "pools": [
   {
    "bonus_rolls": 0,
    "entries": [
     {
      "children": [
       {
        "condition": "minecraft:tool/can_silk_touch",
        "name": "minecraft:deepslate_redstone_ore",
        "type": "minecraft:item"
       },
       {
        "modifier": [
         {
          "add": false,
          "count": {
           "max": 2,
           "min": 1,
           "type": "minecraft:uniform"
          },
          "type": "minecraft:set_count"
         },
         {
          "enchantment": "minecraft:fortune",
          "formula": "minecraft:ore_drops",
          "type": "minecraft:apply_bonus"
         },
         {
          "type": "minecraft:explosion_decay"
         }
        ],
        "name": "spelunkery:rough_cinnabar",
        "type": "minecraft:item"
       }
      ],
      "type": "minecraft:alternatives"
     }
    ],
    "rolls": 1
   },
   {
    "bonus_rolls": 0,
    "entries": [
     {
      "children": [
       {
        "condition": "minecraft:tool/can_silk_touch",
        "type": "minecraft:empty"
       },
       {
        "modifier": [
         {
          "add": false,
          "count": {
           "max": 6,
           "min": 3,
           "type": "minecraft:uniform"
          },
          "type": "minecraft:set_count"
         },
         {
          "enchantment": "minecraft:fortune",
          "formula": "minecraft:ore_drops",
          "type": "minecraft:apply_bonus"
         },
         {
          "type": "minecraft:explosion_decay"
         }
        ],
        "name": "spelunkery:rough_cinnabar_shard",
        "type": "minecraft:item"
       }
      ],
      "type": "minecraft:alternatives"
     }
    ],
    "rolls": 1
   }
  ],
  "random_sequence": "minecraft:blocks/deepslate_redstone_ore",
  "type": "minecraft:block"
 },
 "minecraft:blocks/diamond_ore": {
  "pools": [
   {
    "bonus_rolls": 0,
    "entries": [
     {
      "children": [
       {
        "condition": "minecraft:tool/can_silk_touch",
        "name": "minecraft:diamond_ore",
        "type": "minecraft:item"
       },
       {
        "modifier": [
         {
          "enchantment": "minecraft:fortune",
          "formula": "minecraft:ore_drops",
          "type": "minecraft:apply_bonus"
         },
         {
          "type": "minecraft:explosion_decay"
         }
        ],
        "name": "spelunkery:rough_diamond",
        "type": "minecraft:item"
       }
      ],
      "type": "minecraft:alternatives"
     }
    ],
    "rolls": 1
   }
  ],
  "random_sequence": "minecraft:blocks/diamond_ore",
  "type": "minecraft:block"
 },
 "minecraft:blocks/emerald_ore": {
  "pools": [
   {
    "bonus_rolls": 0,
    "entries": [
     {
      "children": [
       {
        "condition": "minecraft:tool/can_silk_touch",
        "name": "minecraft:emerald_ore",
        "type": "minecraft:item"
       },
       {
        "modifier": [
         {
          "enchantment": "minecraft:fortune",
          "formula": "minecraft:ore_drops",
          "type": "minecraft:apply_bonus"
         },
         {
          "type": "minecraft:explosion_decay"
         }
        ],
        "name": "spelunkery:rough_emerald",
        "type": "minecraft:item"
       }
      ],
      "type": "minecraft:alternatives"
     }
    ],
    "rolls": 1
   }
  ],
  "random_sequence": "minecraft:blocks/emerald_ore",
  "type": "minecraft:block"
 },
 "minecraft:blocks/lapis_ore": {
  "pools": [
   {
    "bonus_rolls": 0,
    "entries": [
     {
      "children": [
       {
        "condition": "minecraft:tool/can_silk_touch",
        "name": "minecraft:lapis_ore",
        "type": "minecraft:item"
       },
       {
        "modifier": [
         {
          "add": false,
          "count": {
           "max": 3,
           "min": 1,
           "type": "minecraft:uniform"
          },
          "type": "minecraft:set_count"
         },
         {
          "enchantment": "minecraft:fortune",
          "formula": "minecraft:ore_drops",
          "type": "minecraft:apply_bonus"
         },
         {
          "type": "minecraft:explosion_decay"
         }
        ],
        "name": "spelunkery:rough_lazurite",
        "type": "minecraft:item"
       }
      ],
      "type": "minecraft:alternatives"
     }
    ],
    "rolls": 1
   }
  ],
  "random_sequence": "minecraft:blocks/lapis_ore",
  "type": "minecraft:block"
 },
 "minecraft:blocks/nether_gold_ore": {
  "pools": [
   {
    "bonus_rolls": 0.0,
    "entries": [
     {
      "children": [
       {
        "condition": "minecraft:tool/can_silk_touch",
        "name": "minecraft:nether_gold_ore",
        "type": "minecraft:item"
       },
       {
        "modifier": [
         {
          "add": false,
          "count": {
           "max": 6.0,
           "min": 2.0,
           "type": "minecraft:uniform"
          },
          "type": "minecraft:set_count"
         },
         {
          "enchantment": "minecraft:fortune",
          "formula": "minecraft:ore_drops",
          "type": "minecraft:apply_bonus"
         },
         {
          "type": "minecraft:explosion_decay"
         }
        ],
        "name": "spelunkery:raw_gold_nugget",
        "type": "minecraft:item"
       }
      ],
      "type": "minecraft:alternatives"
     }
    ],
    "rolls": 1.0
   }
  ],
  "random_sequence": "minecraft:blocks/nether_gold_ore",
  "type": "minecraft:block"
 },
 "minecraft:blocks/redstone_ore": {
  "pools": [
   {
    "bonus_rolls": 0,
    "entries": [
     {
      "children": [
       {
        "condition": "minecraft:tool/can_silk_touch",
        "name": "minecraft:redstone_ore",
        "type": "minecraft:item"
       },
       {
        "modifier": [
         {
          "add": false,
          "count": {
           "max": 2,
           "min": 1,
           "type": "minecraft:uniform"
          },
          "type": "minecraft:set_count"
         },
         {
          "enchantment": "minecraft:fortune",
          "formula": "minecraft:ore_drops",
          "type": "minecraft:apply_bonus"
         },
         {
          "type": "minecraft:explosion_decay"
         }
        ],
        "name": "spelunkery:rough_cinnabar",
        "type": "minecraft:item"
       }
      ],
      "type": "minecraft:alternatives"
     }
    ],
    "rolls": 1
   }
  ],
  "random_sequence": "minecraft:blocks/redstone_ore",
  "type": "minecraft:block"
 },
 "minecraft:blocks/sculk": {
  "pools": [
   {
    "bonus_rolls": 0,
    "entries": [
     {
      "children": [
       {
        "condition": "minecraft:tool/can_silk_touch",
        "name": "minecraft:sculk",
        "type": "minecraft:item"
       },
       {
        "condition": {
         "predicate": {
          "items": [
           "minecraft:shears"
          ]
         },
         "type": "minecraft:match_tool"
        },
        "modifier": [
         {
          "count": {
           "max": 2,
           "min": 0
          },
          "type": "minecraft:set_count"
         }
        ],
        "name": "minecraft:sculk_vein",
        "type": "minecraft:item"
       }
      ],
      "type": "minecraft:alternatives"
     }
    ],
    "rolls": 1
   }
  ],
  "random_sequence": "minecraft:blocks/sculk",
  "type": "minecraft:block"
 },
 "minecraft:blocks/sculk_vein": {
  "pools": [
   {
    "bonus_rolls": 0,
    "entries": [
     {
      "condition": {
       "terms": [
        {
         "predicate": {
          "enchantments": [
           {
            "enchantment": "minecraft:silk_touch",
            "levels": {
             "min": 1
            }
           }
          ]
         },
         "type": "minecraft:match_tool"
        },
        {
         "predicate": {
          "items": [
           "minecraft:shears"
          ]
         },
         "type": "minecraft:match_tool"
        }
       ],
       "type": "minecraft:any_of"
      },
      "modifier": [
       {
        "add": true,
        "condition": {
         "block": "minecraft:sculk_vein",
         "properties": {
          "down": "true"
         },
         "type": "minecraft:block_state_property"
        },
        "count": 1,
        "type": "minecraft:set_count"
       },
       {
        "add": true,
        "condition": {
         "block": "minecraft:sculk_vein",
         "properties": {
          "up": "true"
         },
         "type": "minecraft:block_state_property"
        },
        "count": 1,
        "type": "minecraft:set_count"
       },
       {
        "add": true,
        "condition": {
         "block": "minecraft:sculk_vein",
         "properties": {
          "north": "true"
         },
         "type": "minecraft:block_state_property"
        },
        "count": 1,
        "type": "minecraft:set_count"
       },
       {
        "add": true,
        "condition": {
         "block": "minecraft:sculk_vein",
         "properties": {
          "south": "true"
         },
         "type": "minecraft:block_state_property"
        },
        "count": 1,
        "type": "minecraft:set_count"
       },
       {
        "add": true,
        "condition": {
         "block": "minecraft:sculk_vein",
         "properties": {
          "west": "true"
         },
         "type": "minecraft:block_state_property"
        },
        "count": 1,
        "type": "minecraft:set_count"
       },
       {
        "add": true,
        "condition": {
         "block": "minecraft:sculk_vein",
         "properties": {
          "east": "true"
         },
         "type": "minecraft:block_state_property"
        },
        "count": 1,
        "type": "minecraft:set_count"
       },
       {
        "add": true,
        "count": -1,
        "type": "minecraft:set_count"
       },
       {
        "type": "minecraft:explosion_decay"
       }
      ],
      "name": "minecraft:sculk_vein",
      "type": "minecraft:item"
     }
    ],
    "rolls": 1
   }
  ],
  "random_sequence": "minecraft:blocks/sculk_vein",
  "type": "minecraft:block"
 },
 "minecraft:gameplay/piglin_bartering": {
  "pools": [
   {
    "bonus_rolls": 0,
    "entries": [
     {
      "modifier": [
       {
        "enchantments": [
         "minecraft:soul_speed"
        ],
        "type": "minecraft:enchant_randomly"
       }
      ],
      "name": "minecraft:book",
      "type": "minecraft:item",
      "weight": 5
     },
     {
      "modifier": [
       {
        "enchantments": [
         "minecraft:soul_speed"
        ],
        "type": "minecraft:enchant_randomly"
       }
      ],
      "name": "minecraft:iron_boots",
      "type": "minecraft:item",
      "weight": 8
     },
     {
      "modifier": [
       {
        "id": "minecraft:fire_resistance",
        "type": "minecraft:set_potion"
       }
      ],
      "name": "minecraft:potion",
      "type": "minecraft:item",
      "weight": 8
     },
     {
      "modifier": [
       {
        "id": "minecraft:fire_resistance",
        "type": "minecraft:set_potion"
       }
      ],
      "name": "minecraft:splash_potion",
      "type": "minecraft:item",
      "weight": 8
     },
     {
      "modifier": [
       {
        "id": "minecraft:water",
        "type": "minecraft:set_potion"
       }
      ],
      "name": "minecraft:potion",
      "type": "minecraft:item",
      "weight": 10
     },
     {
      "modifier": [
       {
        "add": false,
        "count": {
         "max": 36,
         "min": 10,
         "type": "minecraft:uniform"
        },
        "type": "minecraft:set_count"
       }
      ],
      "name": "minecraft:iron_nugget",
      "type": "minecraft:item",
      "weight": 10
     },
     {
      "modifier": [
       {
        "add": false,
        "count": {
         "max": 4,
         "min": 2,
         "type": "minecraft:uniform"
        },
        "type": "minecraft:set_count"
       }
      ],
      "name": "minecraft:ender_pearl",
      "type": "minecraft:item",
      "weight": 10
     },
     {
      "modifier": [
       {
        "add": false,
        "count": {
         "max": 9,
         "min": 3,
         "type": "minecraft:uniform"
        },
        "type": "minecraft:set_count"
       }
      ],
      "name": "minecraft:string",
      "type": "minecraft:item",
      "weight": 20
     },
     {
      "modifier": [
       {
        "add": false,
        "count": {
         "max": 12,
         "min": 5,
         "type": "minecraft:uniform"
        },
        "type": "minecraft:set_count"
       }
      ],
      "name": "minecraft:quartz",
      "type": "minecraft:item",
      "weight": 20
     },
     {
      "name": "minecraft:obsidian",
      "type": "minecraft:item",
      "weight": 40
     },
     {
      "name": "minecraft:fire_charge",
      "type": "minecraft:item",
      "weight": 40
     },
     {
      "modifier": [
       {
        "add": false,
        "count": {
         "max": 4,
         "min": 2,
         "type": "minecraft:uniform"
        },
        "type": "minecraft:set_count"
       }
      ],
      "name": "minecraft:leather",
      "type": "minecraft:item",
      "weight": 40
     },
     {
      "modifier": [
       {
        "add": false,
        "count": {
         "max": 8,
         "min": 2,
         "type": "minecraft:uniform"
        },
        "type": "minecraft:set_count"
       }
      ],
      "name": "minecraft:soul_sand",
      "type": "minecraft:item",
      "weight": 40
     },
     {
      "modifier": [
       {
        "add": false,
        "count": {
         "max": 8,
         "min": 2,
         "type": "minecraft:uniform"
        },
        "type": "minecraft:set_count"
       }
      ],
      "name": "minecraft:nether_brick",
      "type": "minecraft:item",
      "weight": 40
     },
     {
      "modifier": [
       {
        "add": false,
        "count": {
         "max": 12,
         "min": 6,
         "type": "minecraft:uniform"
        },
        "type": "minecraft:set_count"
       }
      ],
      "name": "minecraft:spectral_arrow",
      "type": "minecraft:item",
      "weight": 40
     },
     {
      "modifier": [
       {
        "add": false,
        "count": {
         "max": 16,
         "min": 8,
         "type": "minecraft:uniform"
        },
        "type": "minecraft:set_count"
       }
      ],
      "name": "minecraft:gravel",
      "type": "minecraft:item",
      "weight": 40
     },
     {
      "modifier": [
       {
        "add": false,
        "count": {
         "max": 16,
         "min": 8,
         "type": "minecraft:uniform"
        },
        "type": "minecraft:set_count"
       }
      ],
      "name": "minecraft:blackstone",
      "type": "minecraft:item",
      "weight": 40
     }
    ],
    "rolls": 1
   }
  ],
  "random_sequence": "minecraft:gameplay/piglin_bartering",
  "type": "minecraft:barter"
 }
}"##;
