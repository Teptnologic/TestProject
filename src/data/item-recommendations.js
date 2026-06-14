// Boots
const SORC_BOOTS = 3020;    // Sorcerer's Shoes
const BERSERKER = 3006;     // Berserker's Greaves
const MERCURY = 3111;       // Mercury's Treads
const STEELCAPS = 3047;     // Plated Steelcaps
const SWIFTNESS = 3009;     // Boots of Swiftness

const AP_BURST = [SORC_BOOTS, 3089, 3135, 4645, 6655, 3157, 3102, 4628, 3118, 2503, 4646];
const AP_ASSASSIN = [SORC_BOOTS, 3152, 3100, 4645, 3089, 3157, 4646, 3135, 2503, 4633];
const AP_BATTLEMAGE = [SORC_BOOTS, 4633, 6653, 3116, 3089, 3157, 3165, 4629, 3102, 3135];
const AD_ASSASSIN = [SWIFTNESS, 3142, 6697, 6698, 3179, 3814, 6694, 6696, 6699, 6676];
const ADC_CRIT = [BERSERKER, 3031, 3036, 3046, 3094, 3097, 6672, 3072, 6673, 3033, 3085];
const AD_BRUISER = [STEELCAPS, MERCURY, 3071, 6333, 3053, 3161, 6610, 3748, 6631, 3074, 3078, 3181];
const TANK = [STEELCAPS, MERCURY, 3068, 3143, 3083, 3084, 3065, 3110, 6665, 323075, 3742, 2502, 6664];
const ON_HIT = [BERSERKER, 3115, 3124, 3153, 3091, 3302, 6672, 3089];

// Champion ID → recommended item IDs
// Long-term: replace with API-fetched data per champion
export const CHAMPION_ITEMS = {
  // AP Burst Mages
  Ahri: AP_BURST,
  Anivia: AP_BURST,
  Annie: AP_BURST,
  AurelionSol: AP_BURST,
  Brand: [...AP_BATTLEMAGE.slice(0, 3), ...AP_BURST.slice(0, 5)],
  Cassiopeia: AP_BATTLEMAGE,
  Heimerdinger: AP_BURST,
  Hwei: AP_BURST,
  Karthus: AP_BURST,
  Lissandra: AP_BURST,
  Lux: AP_BURST,
  Malzahar: [6653, 3116, 3089, 3135, 3157, 2503, 3102],
  Neeko: AP_BURST,
  Orianna: AP_BURST,
  Ryze: [3003, 6657, 3089, 3135, 3157, 4629, 3102],
  Syndra: AP_BURST,
  Taliyah: AP_BURST,
  Veigar: AP_BURST,
  Velkoz: AP_BURST,
  Vex: AP_BURST,
  Viktor: AP_BURST,
  Xerath: AP_BURST,
  Ziggs: AP_BURST,
  Zoe: AP_BURST,
  Zyra: [6653, 3116, 3089, 3135, 3157, 3165],

  // AP Assassins
  Akali: AP_ASSASSIN,
  Aurora: AP_ASSASSIN,
  Diana: [3152, 3115, 4645, 3089, 3157, 3135],
  Ekko: AP_ASSASSIN,
  Elise: AP_ASSASSIN,
  Evelynn: [3152, 3100, 3089, 4645, 3135, 4646],
  Fizz: AP_ASSASSIN,
  Katarina: [3152, 3115, 3089, 4645, 3157, 3135, 3146],
  Leblanc: AP_ASSASSIN,
  Nidalee: AP_ASSASSIN,
  Sylas: [3152, 3100, 3089, 3157, 4645, 3135, 4629],
  Kassadin: [6657, 3089, 3100, 3157, 3135, 4645, 3003],

  // AP Battlemages
  Vladimir: AP_BATTLEMAGE,
  Mordekaiser: [4633, 3116, 3089, 3157, 6653, 3135, 3165],
  Rumble: AP_BATTLEMAGE,
  Swain: [6653, 3116, 4633, 3157, 3089, 3165],
  Singed: [4633, 3116, 3089, 3157, 6653],
  Gragas: AP_BATTLEMAGE,
  Lillia: [6653, 3116, 4629, 3089, 3157, 3135],

  // AP On-hit / unique AP builds
  Kayle: [3115, 3089, 4633, 3124, 3135, 3157],
  Teemo: [3115, 6653, 3089, 3116, 3135, 3157],
  Azir: [3115, 3089, 3135, 3157, 4645, 4629],
  Gwen: [4633, 3115, 3089, 3157, 3135, 4629],
  Kennen: [3152, 3157, 3089, 3135, 4645, 3116],

  // AD Assassins
  Zed: AD_ASSASSIN,
  Talon: AD_ASSASSIN,
  Khazix: AD_ASSASSIN,
  Qiyana: AD_ASSASSIN,
  Naafiri: AD_ASSASSIN,
  Shaco: AD_ASSASSIN,
  Rengar: [6697, 6698, 3142, 3814, 6694, 6333, 3074],
  Pyke: [3142, 6697, 3179, 3814, 6694, 6699],

  // ADC / Crit Marksmen
  Aphelios: ADC_CRIT,
  Ashe: ADC_CRIT,
  Caitlyn: ADC_CRIT,
  Draven: [3072, 3031, 3036, 6673, 3033, 3097],
  Jinx: ADC_CRIT,
  Jhin: [3031, 3094, 3036, 3097, 6673, 6676, 3033],
  Kaisa: [3115, 3031, 3046, 3094, 6672, 3085],
  Kalista: [6672, 3153, 3085, 3031, 3036],
  KogMaw: [3124, 3153, 3091, 3085, 6672, 3115],
  Lucian: [3031, 6672, 3508, 3036, 6673, 3046],
  MissFortune: [3031, 6676, 3036, 3033, 3097, 6694],
  Samira: [3031, 3072, 6673, 3036, 3033],
  Sivir: ADC_CRIT,
  Tristana: ADC_CRIT,
  Twitch: ADC_CRIT,
  Vayne: [6672, 3153, 3124, 3046, 3031, 3036, 3091],
  Xayah: ADC_CRIT,
  Zeri: ADC_CRIT,
  Smolder: [3031, 3036, 3094, 3508, 6673, 3033],
  Yunara: ADC_CRIT,
  Quinn: [3142, 3031, 6676, 3036, 6697, 3097],
  Akshan: [6672, 3031, 3036, 3094, 3091, 3046],
  Kindred: [6672, 3031, 3036, 3153, 3046, 3085],
  Graves: [3142, 6676, 3031, 3036, 3072, 6697],
  Nilah: [3031, 3072, 6673, 3036, 3046, 3153],

  // Hybrid / Unique
  Corki: [3078, 3031, 3094, 3036, 3033, 3097],
  Ezreal: [3004, 3078, 3100, 6694, 3508, 6694],
  Jayce: [3142, 6694, 3036, 3071, 3814, 6697],
  Varus: [3004, 3031, 3036, 3085, 3115, 6672],
  TwistedFate: [3115, 3100, 3089, 4645, 3157, 3135],

  // AD Bruisers / Fighters
  Aatrox: AD_BRUISER,
  Ambessa: AD_BRUISER,
  Belveth: [3153, 3124, 6672, 3031, 3302, 3091],
  Camille: [3078, 3074, 6333, 3053, 3161, 6610],
  Darius: [3078, 3053, 6333, 3071, 6610, 3742],
  Fiora: [3074, 6333, 3078, 3161, 3053, 3181],
  Gangplank: [3078, 3031, 3508, 6694, 3036, 3074],
  Garen: [3078, 3053, 6333, 3071, 3181, 6610],
  Hecarim: [3078, 3053, 6333, 3161, 6610, 3071],
  Illaoi: [3071, 3053, 6333, 6610, 3161, 3748],
  Irelia: [3153, 3078, 3053, 6333, 3091, 6631],
  Jax: [3078, 3153, 3053, 6631, 6333, 3161],
  JarvanIV: [3071, 6610, 3053, 6333, 3161, 6631],
  Kled: [3071, 6610, 3053, 6333, 3748, 3181],
  MasterYi: ON_HIT,
  Nasus: [3078, 3053, 6333, 3742, 3065, 3110],
  Nocturne: [6631, 3153, 3071, 6333, 3053, 3742],
  Olaf: [3071, 6333, 3053, 6631, 3161, 3153],
  Pantheon: [3071, 6333, 3814, 3053, 6610, 3161],
  Kayn: [3071, 6333, 3053, 3142, 6697, 6694],
  LeeSin: [3071, 6333, 3053, 6610, 3161, 3142],
  Riven: [3071, 6333, 3161, 6610, 3053, 3142],
  Renekton: [3071, 6333, 3053, 6610, 3161, 3748],
  Sett: [3071, 3053, 6333, 3748, 6610, 3161],
  Tryndamere: [3031, 3046, 6673, 3036, 3072, 3153],
  Vi: [3078, 3071, 6333, 3053, 6610, 3161],
  Viego: [3078, 3153, 6333, 3053, 3071, 6631],
  MonkeyKing: [3071, 6333, 3053, 6610, 3161, 3748],
  XinZhao: [3078, 3153, 3053, 6631, 6333, 3071],
  Yasuo: [3031, 3153, 6673, 3036, 6333, 3046],
  Yone: [3031, 3153, 6673, 3036, 6333, 3046],
  Yorick: [3078, 3748, 3181, 6610, 3053, 3071],
  Zaahen: AD_BRUISER,
  Urgot: [3071, 3748, 3153, 6631, 6333, 3053],
  Warwick: [3153, 3748, 3078, 6631, 3053, 3065],
  Trundle: [3153, 3748, 3078, 6631, 3053, 3065],
  Volibear: [3748, 3078, 3053, 3068, 3065, 3084],

  // Tanks
  Alistar: TANK,
  Amumu: TANK,
  Braum: TANK,
  Chogath: [3084, 3068, 3143, 3065, 6665, 3083],
  DrMundo: [3083, 3065, 3084, 3068, 3143, 6665],
  Galio: [3068, 3157, 3102, 3143, 6665, 3065],
  Gnar: [3071, 3053, 3143, 3742, 3068, 6665],
  KSante: [6665, 3068, 3143, 3065, 3742, 3110],
  Leona: TANK,
  Malphite: [3068, 3110, 3143, 6665, 3742, 3084],
  Maokai: TANK,
  Nautilus: TANK,
  Nunu: TANK,
  Ornn: [3068, 3110, 3143, 6665, 3065, 3742],
  Poppy: [3068, 3143, 6665, 3742, 3110, 3084],
  Rammus: [323075, 3068, 3143, 3742, 3110, 6665],
  RekSai: [3071, 6333, 3053, 6610, 3161, 3068],
  Rell: TANK,
  Sejuani: TANK,
  Shen: [3068, 3748, 3143, 6665, 3065, 3742],
  Shyvana: [3115, 4633, 3089, 3068, 3143, 3153],
  Sion: [3084, 3068, 3143, 3083, 3742, 6665],
  Skarner: TANK,
  TahmKench: [3084, 3068, 3143, 3065, 6665, 3083],
  Udyr: [3068, 3748, 3065, 6665, 3742, 3084],
  Zac: TANK,
  Blitzcrank: TANK,
};
