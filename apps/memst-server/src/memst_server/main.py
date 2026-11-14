"""MemSt API Server - Main entry point."""

import uvicorn
from fastapi import FastAPI
from fastapi.middleware.cors import CORSMiddleware

from memst_server.config import get_config
from memst_server.api import routes
from memst_server.kg_extraction_service import get_kg_extraction_service


def create_app() -> FastAPI:
    """Create and configure FastAPI application."""
    config = get_config()
    
    # Initialize KG extraction service with config
    if config.kg_extraction.enabled:
        kg_service = get_kg_extraction_service(config.kg_extraction.db_path)
        if kg_service.is_available:
            print(f"KG Extraction v2 enabled (db: {config.kg_extraction.db_path or 'in-memory'})")

    app = FastAPI(
        title="MemSt API",
        description="Hybrid session memory system for LLM applications",
        version="0.1.0",
        docs_url="/docs",
        redoc_url="/redoc",
    )

    # Add CORS middleware
    # Use ["*"] for development if no origins specified, otherwise use config
    cors_origins = config.server.cors_origins if config.server.cors_origins else ["*"]
    app.add_middleware(
        CORSMiddleware,
        allow_origins=cors_origins,
        allow_credentials=True,
        allow_methods=["*"],
        allow_headers=["*"],
    )

    # Include API routes
    app.include_router(routes.router)

    @app.get("/")
    def root():
        return {
            "name": "MemSt API Server",
            "version": "0.1.0",
            "docs": "/docs",
        }

    return app


def main():
    """Run the server."""
    config = get_config()
    app = create_app()

    print(f"Starting MemSt API Server on {config.server.host}:{config.server.port}")
    print(f"MemSt store path: {config.server.store_path}")

    uvicorn.run(
        app,
        host=config.server.host,
        port=config.server.port,
        reload=config.server.debug,
    )


if __name__ == "__main__":
    main()
