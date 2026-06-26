import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

// Dev-only endpoint that prints client-side logs to the terminal running `npm run dev`.
function serverLogPlugin() {
  return {
    name: 'server-log',
    configureServer(server) {
      server.middlewares.use('/__log', (req, res) => {
        if (req.method !== 'POST') { res.statusCode = 405; res.end(); return; }
        let body = '';
        req.on('data', (chunk) => { body += chunk; });
        req.on('end', () => {
          try {
            const { label, data } = JSON.parse(body);
            console.log(`\n[${label}]`);
            console.dir(data, { depth: 4, colors: true });
          } catch {}
          res.statusCode = 204;
          res.end();
        });
      });
    },
  };
}

// https://vite.dev/config/
export default defineConfig({
  plugins: [react(), serverLogPlugin()],
})
