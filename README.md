# iCONup

Utility desktop per l'installazione dei prodotti della **iCON Suite** via FTP/SFTP.

## Funzionalità

- Upload via **FTP** e **SFTP** (SSH)
- Progress bar in tempo reale
- Gestione profili clienti multipli
- Crittografia AES-256 per i profili salvati
- Esporta/Importa profili

## Prodotti supportati

- iCONcms - Visual HTML Editor
- iCONstat - Analytics Dashboard
- iCONblog - Blog System
- iCONvert - HTML Converter
- Percorso personalizzato

## Sicurezza

Le credenziali sono salvate localmente in forma crittografata (AES-256-GCM). Non vengono mai inviate a server esterni.

## Build

Richiede Node.js 18+ e Rust 1.70+

```bash
npm install
npm run build
```

## Licenza

MIT © YMEDIA
