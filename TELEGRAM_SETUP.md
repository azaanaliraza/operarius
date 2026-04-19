# Telegram Bot Setup Guide for Operarius

This guide walks you through setting up Operarius as a Telegram bot for messaging and conversational AI.

## Prerequisites

- A Telegram account
- Operarius application installed and running
- The Llama.cpp inference server running on port 8080

## Step 1: Create a Telegram Bot via @BotFather

1. Open **Telegram** and search for **@BotFather** (or visit [t.me/BotFather](https://t.me/BotFather))
2. Send `/newbot` to create a new bot
3. Choose a **display name** (e.g., "Operarius Bot") — can be anything
4. Choose a **unique username** ending in `bot` (e.g., `operarius_bot`)
5. **BotFather** replies with your bot token — it looks like:
   ```
   123456789:ABCdefGHIjklMNOpqrSTUvwxYZ
   ```
6. **SAVE THIS TOKEN** — you'll need it in the next step

> ⚠️ **Keep your bot token secret!** Anyone with this token can control your bot.

## Step 2: Configure Bot Settings (Optional)

To improve user experience, configure these BotFather commands:

1. Message **@BotFather** and send `/setcommands`
2. When prompted, paste:
   ```
   help - Show help information
   new - Start a new conversation
   status - Check bot status
   ```

Other useful commands via @BotFather:
- `/setdescription` - Add a description shown before users start chatting
- `/setabouttext` - Add text on the bot's profile
- `/setuserpic` - Upload an avatar

## Step 3: Configure Operarius

1. Open the **Operarius** application
2. Navigate to the **Settings** or **Connect** panel
3. Paste your bot token in the **Telegram Bot Token** field
4. Click **Connect Telegram**

Operarius will:
- Save the token securely
- Configure the Hermes gateway to use the token
- Start the Telegram connection automatically

## Step 4: Test Your Bot

1. Open **Telegram** and search for your bot username
2. Click **START** or send `/help`
3. Send a test message like **"hello"**
4. Your bot should respond within 2-5 seconds

Expected response: A helpful message from Operarius

### Troubleshooting

| Issue | Solution |
|-------|----------|
| Bot doesn't respond | Check that Operarius is running and the inference server is online |
| Bot responds slowly (>10s) | The inference server may be overloaded. Try a shorter message. |
| Token validation failed | Double-check the token from @BotFather. It should be exactly as shown. |
| "Privacy mode" issues in groups | Disable privacy mode in @BotFather if using the bot in group chats |

## Step 5: Use in Groups (Optional)

To use Operarius in a Telegram group:

1. Add the bot to your group
2. In @BotFather, send `/setprivacy` and select the bot
3. Choose **Disabled** to let the bot see all group messages
4. Remove and re-add the bot to the group for the setting to take effect

Once enabled, the bot will respond to:
- Direct mentions: `@operarius_bot what's 2+2?`
- Replies to the bot's own messages
- Messages starting with `/` commands

## Architecture

When you configure your Telegram bot token in Operarius:

```
You (Telegram)
    ↓
Telegram API
    ↓
Operarius (Hermes Gateway)
    ↓
Llama.cpp Inference Server (Port 8080)
    ↓
Response (2-5 seconds)
```

- **Telegram API** is Telegram's official platform
- **Hermes Gateway** is the messaging router that handles all platforms
- **Llama.cpp** is the local AI inference engine

## Advanced Configuration

For advanced users, Telegram settings can be configured in `~/.operarius/hermes/.env`:

```bash
# Required
TELEGRAM_BOT_TOKEN=your_token_here

# Optional
TELEGRAM_ALLOWED_USERS=123456789,987654321  # Comma-separated user IDs
TELEGRAM_HOME_CHANNEL=-1001234567890        # Chat ID for scheduled task results
```

Find your Telegram user ID by messaging [@userinfobot](https://t.me/userinfobot).

## Voice Messages (Coming Soon)

Operarius will soon support:
- **Voice input**: Send voice memos on Telegram → transcribed to text → AI response
- **Voice output**: AI responses delivered as Telegram voice messages

## FAQ

**Q: Can I run the bot from my laptop?**  
A: Yes! Operarius uses long polling, so it works from anywhere with internet.

**Q: Can I use the bot in multiple chats?**  
A: Yes, one bot can handle unlimited chats and groups.

**Q: What data does Operarius save?**  
A: Only your conversation history in the local database. No data is sent to Telegram or external servers (everything runs locally on your machine).

**Q: Can I have multiple bots?**  
A: Yes, create multiple bots via @BotFather and add them each as separate connections.

## Support

For issues or feature requests, check the main [Operarius README](./README.md) or file an issue on GitHub.
