import { NextResponse } from "next/server";
import fs from "fs";
import { Client as SSHClient } from "ssh2";

export async function POST(request: Request) {
  const body = await request.json();
  const { sshHost, sshPort, sshUser, sshKeyPath, sshKeyPassphrase, basePath } = body;

  if (!sshHost || !sshUser) {
    return NextResponse.json(
      { ok: false, error: "Host and user are required" },
      { status: 400 }
    );
  }

  const connectConfig: Record<string, unknown> = {
    host: sshHost,
    port: sshPort || 22,
    username: sshUser,
    readyTimeout: 10000,
  };

  if (sshKeyPath) {
    try {
      connectConfig.privateKey = fs.readFileSync(sshKeyPath);
      if (sshKeyPassphrase) {
        connectConfig.passphrase = sshKeyPassphrase;
      }
    } catch {
      return NextResponse.json(
        { ok: false, error: `Cannot read SSH key: ${sshKeyPath}` },
        { status: 400 }
      );
    }
  }

  try {
    await new Promise<void>((resolve, reject) => {
      const conn = new SSHClient();
      const timeout = setTimeout(() => {
        conn.end();
        reject(new Error("Connection timed out after 10s"));
      }, 12000);

      conn.on("ready", () => {
        // If basePath provided, check it exists via SFTP
        if (basePath) {
          conn.sftp((err, sftp) => {
            if (err) {
              clearTimeout(timeout);
              conn.end();
              reject(new Error(`SFTP error: ${err.message}`));
              return;
            }
            sftp.stat(basePath, (statErr) => {
              clearTimeout(timeout);
              conn.end();
              if (statErr) {
                reject(
                  new Error(`Base path not found on remote: ${basePath}`)
                );
              } else {
                resolve();
              }
            });
          });
        } else {
          clearTimeout(timeout);
          conn.end();
          resolve();
        }
      });

      conn.on("error", (err) => {
        clearTimeout(timeout);
        reject(err);
      });

      conn.connect(connectConfig);
    });

    return NextResponse.json({ ok: true });
  } catch (err) {
    const message = err instanceof Error ? err.message : "Connection failed";
    return NextResponse.json({ ok: false, error: message });
  }
}
