# Gemini-TG-Assistant

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE-MIT)
[![License: Apache 2.0](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](LICENSE-APACHE)
[![Rust 1.92.0](https://img.shields.io/badge/rust-1.90+-orange.svg)](https://www.rust-lang.org/)

A lightweight, no-nonsense Telegram assistant using Google's Gemini AI. Focused on simplicity, context summarization, and clean architecture.

## Project Status

🚀 **Work in Progress:** For a detailed list of planned features and current progress, please check our [Roadmap](ROADMAP.md).

## Architecture

The bot leverages a polling mechanism via teloxide to capture incoming updates. All messages, including metadata regarding replies, quotes, and forwards, are persisted in a database to ensure structural context is maintained.

Key components:
1. Message Processing & Storage: Captures full interaction history, preserving the thread structure and message relationships.
2. Context Management: On incoming requests (private chats or direct mentions), the system retrieves the relevant thread history from the database.
3. Summarization Engine: Periodically condenses historical data into summaries to optimize token usage and prevent context window exhaustion.
4. Model Interaction: The aggregated context, enriched with metadata, is sent to the Gemini API with instructions to continue the dialogue based on the established conversation flow.
5. Content Filtering: Currently, messages with media are excluded from the processing pipeline, focusing exclusively on textual data.

## License

Licensed under either of Apache License 2.0 or MIT at your option.
