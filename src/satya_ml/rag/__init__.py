"""Hybrid PyLance BM25/FTS + Semantic Vector RAG over Rules, Rust Engine & Lore."""

from satya_ml.rag.corpus_builder import (
    build_all_knowledge_chunks,
    build_campaign_lore_chunks,
    build_rust_engine_symbol_chunks,
    build_ttrpg_rules_chunks,
)
from satya_ml.rag.hybrid_index import (
    EMBEDDING_DIM,
    SatyaHybridRagIndex,
    embed_rag_text,
    get_satya_rag_index,
    reset_rag_index_cache,
)

__all__ = [
    "EMBEDDING_DIM",
    "SatyaHybridRagIndex",
    "build_all_knowledge_chunks",
    "build_campaign_lore_chunks",
    "build_rust_engine_symbol_chunks",
    "build_ttrpg_rules_chunks",
    "embed_rag_text",
    "get_satya_rag_index",
    "reset_rag_index_cache",
]
