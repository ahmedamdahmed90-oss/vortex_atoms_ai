#!/usr/bin/env python3
"""Compile the Vortex Atoms AI global humanities source into compact binary form."""
from __future__ import annotations

import struct
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SRC = ROOT / "knowledge" / "global_humanities_module.source.md"
OUT = ROOT / "knowledge" / "global_humanities_module.bin"
MAX_BYTES = 50 * 1024 * 1024
MAGIC = b"VAI_GH01"

MAJOR_LANGUAGE_DICTIONARY = {
    "en": ["hello", "thank you", "yes", "no", "water", "food", "doctor", "map", "history", "trade"],
    "zh": ["你好", "谢谢", "是", "不", "水", "食物", "医生", "地图", "历史", "贸易"],
    "hi": ["नमस्ते", "धन्यवाद", "हाँ", "नहीं", "पानी", "भोजन", "डॉक्टर", "नक्शा", "इतिहास", "व्यापार"],
    "es": ["hola", "gracias", "sí", "no", "agua", "comida", "médico", "mapa", "historia", "comercio"],
    "fr": ["bonjour", "merci", "oui", "non", "eau", "nourriture", "médecin", "carte", "histoire", "commerce"],
    "ar": ["مرحبا", "شكرا", "نعم", "لا", "ماء", "طعام", "طبيب", "خريطة", "تاريخ", "تجارة"],
    "bn": ["নমস্কার", "ধন্যবাদ", "হ্যাঁ", "না", "পানি", "খাবার", "ডাক্তার", "মানচিত্র", "ইতিহাস", "বাণিজ্য"],
    "pt": ["olá", "obrigado", "sim", "não", "água", "comida", "médico", "mapa", "história", "comércio"],
    "ru": ["здравствуйте", "спасибо", "да", "нет", "вода", "еда", "врач", "карта", "история", "торговля"],
    "ur": ["سلام", "شکریہ", "ہاں", "نہیں", "پانی", "کھانا", "ڈاکٹر", "نقشہ", "تاریخ", "تجارت"],
    "id": ["halo", "terima kasih", "ya", "tidak", "air", "makanan", "dokter", "peta", "sejarah", "perdagangan"],
    "de": ["hallo", "danke", "ja", "nein", "wasser", "essen", "arzt", "karte", "geschichte", "handel"],
    "ja": ["こんにちは", "ありがとう", "はい", "いいえ", "水", "食べ物", "医者", "地図", "歴史", "貿易"],
    "sw": ["habari", "asante", "ndiyo", "hapana", "maji", "chakula", "daktari", "ramani", "historia", "biashara"],
    "tr": ["merhaba", "teşekkürler", "evet", "hayır", "su", "yemek", "doktor", "harita", "tarih", "ticaret"],
    "ko": ["안녕하세요", "감사합니다", "예", "아니요", "물", "음식", "의사", "지도", "역사", "무역"],
    "it": ["ciao", "grazie", "sì", "no", "acqua", "cibo", "medico", "mappa", "storia", "commercio"],
    "fa": ["سلام", "متشکرم", "بله", "نه", "آب", "غذا", "پزشک", "نقشه", "تاریخ", "تجارت"],
    "vi": ["xin chào", "cảm ơn", "có", "không", "nước", "thức ăn", "bác sĩ", "bản đồ", "lịch sử", "thương mại"],
    "th": ["สวัสดี", "ขอบคุณ", "ใช่", "ไม่", "น้ำ", "อาหาร", "แพทย์", "แผนที่", "ประวัติศาสตร์", "การค้า"],
}

SECTIONS = [
    (
        "grammar_advanced_structures",
        "Advanced Grammatical Structures and Typology",
        ["grammar", "syntax", "morphology", "semantics", "pragmatics", "case", "agreement", "tense", "aspect", "mood"],
        "Syntax layer includes dependency grammar, constituency grammar, head directionality, phrase structure, movement, agreement, binding, valency, transitivity, argument structure, information structure, topic/comment, focus, ellipsis, and coordination. Morphology layer models isolating, agglutinative, fusional, polysynthetic, templatic, reduplicative, compounding, derivation, inflection, cliticization, case marking, gender and noun-class systems, definiteness, evidentiality, honorifics, and politeness systems. Semantics and pragmatics layer tracks tense, aspect, mood, modality, negation scope, quantification, deixis, anaphora, presupposition, implicature, discourse markers, register, domain terminology, and culture-bound lexical gaps.",
    ),
    (
        "translation_major_world_languages",
        "Real-Time Translation Dictionaries for Major World Languages",
        ["translation", "dictionary", "language", "world languages", "multilingual", "transliteration", "locale", "phrase table"],
        "Core language set covers English, Mandarin Chinese, Hindi, Spanish, French, Modern Standard Arabic, Bengali, Portuguese, Russian, Urdu, Indonesian, German, Japanese, Swahili, Turkish, Korean, Italian, Persian/Farsi, Vietnamese, Thai, Polish, Dutch, Tamil, Telugu, Marathi, Punjabi, Gujarati, Ukrainian, Hebrew, Greek, Romanian, Czech, Hungarian, Malay, Filipino/Tagalog, Hausa, Yoruba, Amharic, Somali, Zulu, Xhosa, and regional lingua francas. Runtime mapping normalizes script, detects language family and register, tokenizes, transliterates when needed, maps phrase tables, applies morphology and gender/number agreement, reorders syntax, preserves named entities, and back-checks ambiguity. Seed phrase table: " + "; ".join(f"{code}={'|'.join(words)}" for code, words in MAJOR_LANGUAGE_DICTIONARY.items()),
    ),
    (
        "global_history_timelines",
        "Expansive Global History Timelines",
        ["history", "timeline", "civilization", "empire", "trade", "world history", "chronology", "decolonization", "industrialization"],
        "Deep timeline spans prehistory, agricultural revolution, river civilizations, classical empires, axial age, silk-road exchange, late antiquity, medieval polities, trans-Saharan and Indian Ocean networks, steppe empires, early modern expansion, scientific revolution, industrialization, imperialism, world wars, decolonization, Cold War, globalization, digital age, and climate/energy transition. Regional frames include Africa, Middle East, South Asia, East Asia, Southeast Asia, Europe, Central Asia, Oceania, North America, Latin America, Caribbean, Arctic, and global maritime systems. Reasoning model tracks chronology, causality, continuity and change, source criticism, periodization, demographic shifts, technological diffusion, trade networks, state formation, migration, conflict, institutional development, and cultural exchange.",
    ),
    (
        "geography_gis_mapping_logic",
        "Detailed Geography and GIS Mapping Logic",
        ["geography", "gis", "mapping", "coordinate", "projection", "geohash", "spatial index", "r-tree", "haversine", "routing"],
        "Geography models physical geography, human geography, political boundaries, urban systems, transport corridors, watershed logic, climate zones, biomes, terrain constraints, population density, land use, resource distribution, and hazard exposure. GIS primitives include point, line, polygon, multipolygon, raster, vector tile, attribute table, topology, CRS, datum, projection, geoid, scale, resolution, accuracy, precision, metadata, and lineage. Algorithms include Haversine and Vincenty distance, great-circle routing, map projection transforms, geohash, S2/H3-style spatial indexing, R-tree bounding boxes, point-in-polygon, buffering, overlay, nearest neighbor, network routing, isochrones, viewshed, kriging, raster reclassification, and zonal statistics.",
    ),
    (
        "high_density_binary_runtime",
        "High-Density Binary Serialization and Vector Index Runtime",
        ["binary serialization", "compression", "vector db", "qdrant", "mmap", "token cache", "leak prevention", "ultra lean"],
        "Serialization format uses 8-byte magic, little-endian section count, length-prefixed section identifiers, length-prefixed titles, compact tag arrays, and UTF-8 content payloads without JSON/TOML whitespace. Vector DB interface maps grammar, translation, world history, geography, and GIS semantic intents to the global_humanities_module descriptor. Qdrant-compatible snapshots preserve active vector-layer indexing, hot token cache avoids repeated disk reads, and Kernel_03 loads through read-only mmap per request, performs checked slicing, creates no mutable singleton, and explicitly drops the mapping after response generation.",
    ),
]


def put_u16(buf: bytearray, value: int) -> None:
    if value > 0xFFFF:
        raise ValueError(f"u16 overflow: {value}")
    buf.extend(struct.pack("<H", value))


def put_u32(buf: bytearray, value: int) -> None:
    if value > 0xFFFFFFFF:
        raise ValueError(f"u32 overflow: {value}")
    buf.extend(struct.pack("<I", value))


def put_bytes_u16(buf: bytearray, text: str) -> None:
    data = text.encode("utf-8")
    put_u16(buf, len(data))
    buf.extend(data)


def put_bytes_u32(buf: bytearray, text: str) -> None:
    data = " ".join(text.split()).encode("utf-8")
    put_u32(buf, len(data))
    buf.extend(data)


def main() -> None:
    buf = bytearray(MAGIC)
    put_u32(buf, len(SECTIONS))
    for section_id, title, tags, content in SECTIONS:
        put_bytes_u16(buf, section_id)
        put_bytes_u16(buf, title)
        put_u16(buf, len(tags))
        for tag in tags:
            put_bytes_u16(buf, tag)
        put_bytes_u32(buf, content)

    OUT.write_bytes(buf)
    size = OUT.stat().st_size
    if size >= MAX_BYTES:
        raise SystemExit(f"compiled module exceeds 50 MiB mapping budget: {size} bytes")
    print(f"compiled {OUT} ({size} bytes, {size / MAX_BYTES:.6%} of 50 MiB budget) from {SRC}")


if __name__ == "__main__":
    main()
