# Website Publishing with RFS

This tutorial will guide you through the process of publishing a website using RFS. You'll learn how to prepare a website directory, publish it to an RFS server, and access it through a web browser.

## Prerequisites

Before you begin, make sure you have:

- RFS installed (see the [Getting Started](./getting-started.md) tutorial)
- An RFS server running (see the [Server Setup](./server-setup.md) tutorial)
- A website directory with HTML, CSS, and other web assets
- Basic understanding of web development and command-line operations

## Understanding Website Publishing

RFS allows you to publish websites by:

1. Uploading the website files to an RFS server
2. Creating an flist from these files
3. Serving the website through the RFS server's web interface

This approach offers several advantages:

- **Efficient distribution**: Only the metadata is transferred initially, with content downloaded on-demand
- **Deduplication**: Identical files are stored only once, saving storage space
- **Versioning**: You can publish multiple versions of your website
- **Caching**: Frequently accessed files are cached for improved performance

## Step 1: Prepare a Sample Website

Let's create a simple website for this tutorial:

```bash
# Create a directory for the website
mkdir -p ~/sample-website

# Create an index.html file
cat > ~/sample-website/index.html << 'EOF'
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>RFS Sample Website</title>
    <link rel="stylesheet" href="styles.css">
</head>
<body>
    <header>
        <h1>Welcome to RFS Website Publishing</h1>
    </header>
    <main>
        <p>This is a sample website published using RFS.</p>
        <img src="images/logo.png" alt="RFS Logo">
    </main>
    <footer>
        <p>&copy; 2025 RFS Project</p>
    </footer>
</body>
</html>
EOF

# Create a CSS file
mkdir -p ~/sample-website/css
cat > ~/sample-website/css/styles.css << 'EOF'
body {
    font-family: Arial, sans-serif;
    line-height: 1.6;
    margin: 0;
    padding: 20px;
    color: #333;
}

header {
    background-color: #f4f4f4;
    padding: 20px;
    text-align: center;
}

main {
    padding: 20px;
}

footer {
    text-align: center;
    padding: 10px;
    background-color: #f4f4f4;
    margin-top: 20px;
}

img {
    max-width: 300px;
    display: block;
    margin: 20px auto;
}
EOF

# Create an images directory and add a placeholder logo
mkdir -p ~/sample-website/images
# You can add your own logo.png file here, or create a placeholder:
echo "This would be a logo image" > ~/sample-website/images/logo.txt
```

## Step 2: Publish the Website

Now, let's publish the website to the RFS server:

```bash
# Publish the website
rfs website-publish ~/sample-website --server http://localhost:3000
```

You should see output similar to:

```
Uploading website files...
Processing directory: /home/user/sample-website
Found 3 files, 3 directories
Processed 3 files, 2.5 KB total
Created 3 unique blocks, 2.5 KB total
Website published successfully!
Website URL: http://localhost:3000/websites/abc123
```

Note the URL provided in the output. This is the URL where your website is now accessible.

## Step 3: Access the Published Website

Open a web browser and navigate to the URL provided in the previous step:

```
http://localhost:3000/websites/abc123
```

You should see your website displayed in the browser.

## Step 4: Update the Website

Let's update the website and publish a new version:

```bash
# Update the index.html file
cat > ~/sample-website/index.html << 'EOF'
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>RFS Sample Website - Updated</title>
    <link rel="stylesheet" href="styles.css">
</head>
<body>
    <header>
        <h1>Welcome to RFS Website Publishing</h1>
    </header>
    <main>
        <p>This is an updated sample website published using RFS.</p>
        <p>Website publishing with RFS is efficient and easy!</p>
        <img src="images/logo.png" alt="RFS Logo">
    </main>
    <footer>
        <p>&copy; 2025 RFS Project - Updated Version</p>
    </footer>
</body>
</html>
EOF

# Publish the updated website
rfs website-publish ~/sample-website --server http://localhost:3000
```

You'll receive a new URL for the updated version of the website.

## Advanced: Publishing with Authentication

If your RFS server requires authentication, you can provide credentials:

```bash
# First, obtain a token
TOKEN=$(curl -X POST http://localhost:3000/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username":"user1","password":"password1"}' | jq -r '.token')

# Then publish with the token
curl -X POST http://localhost:3000/websites \
  -H "Authorization: Bearer $TOKEN" \
  -F "file=@~/sample-website/index.html" \
  -F "file=@~/sample-website/css/styles.css" \
  -F "file=@~/sample-website/images/logo.txt"
```

## Advanced: Custom Domain Configuration

To use a custom domain with your published website, you'll need to:

1. Configure your domain's DNS to point to your RFS server
2. Configure your RFS server to handle requests for your domain

This typically involves setting up a reverse proxy like Nginx or Caddy:

```nginx
# Example Nginx configuration
server {
    listen 80;
    server_name your-domain.com;

    location / {
        proxy_pass http://localhost:3000/websites/abc123;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
    }
}
```

## Troubleshooting

### Website Not Accessible

If your website is not accessible:

1. Check that the RFS server is running
2. Verify the URL is correct
3. Check the server logs for errors
4. Ensure all website files were uploaded successfully

### File Not Found Errors

If you see "File Not Found" errors:

1. Check that all referenced files (CSS, images, etc.) were included in the upload
2. Verify that file paths in your HTML are correct
3. Check for case sensitivity issues in file paths

### Server Authentication Issues

If you encounter authentication issues:

1. Verify your username and password
2. Check that your token is valid and not expired
3. Ensure you're including the token correctly in the Authorization header

## Next Steps

Now that you've learned how to publish websites with RFS, you might want to:

- Learn about [syncing files between servers](./syncing-files.md)
- Explore [advanced server configuration](../user-guides/fl-server.md)
- Set up [multiple storage backends](../concepts/stores.md) for redundancy
- Learn about [performance tuning](../user-guides/performance-tuning.md) for high-traffic websites