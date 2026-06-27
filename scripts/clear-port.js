import { execSync, spawn } from 'child_process';

const PORT = 1420;

// 清理残留进程
function cleanProcesses() {
  try {
    if (process.platform === 'win32') {
      // 1. 强行结束所有残留的 tauri-app.exe 进程
      try {
        execSync('taskkill /F /IM tauri-app.exe', { stdio: 'ignore' });
      } catch (e) {
        // 忽略找不到进程的错误
      }

      // 2. 查找并结束占用指定端口的进程
      let output = '';
      try {
        output = execSync(`netstat -ano | findstr :${PORT}`, { encoding: 'utf8' });
      } catch (e) {
        // 忽略没有找到占用的错误
      }

      if (output) {
        const lines = output.split('\n');
        const pids = new Set();
        for (const line of lines) {
          const parts = line.trim().split(/\s+/);
          if (parts.length >= 5) {
            const pid = parts[4];
            if (pid && pid !== '0') {
              pids.add(pid);
            }
          }
        }
        for (const pid of pids) {
          try {
            execSync(`taskkill /F /PID ${pid}`, { stdio: 'ignore' });
            console.log(`[clear-port] Killed process ${pid} on port ${PORT}`);
          } catch (e) {
            // 忽略可能已退出的进程
          }
        }
      }
    } else {
      // Unix 平台清理
      try {
        execSync('pkill -9 tauri-app', { stdio: 'ignore' });
      } catch (e) {}
      try {
        const pids = execSync(`lsof -t -i:${PORT}`, { encoding: 'utf8' }).trim().split('\n');
        for (const pid of pids) {
          if (pid) {
            execSync(`kill -9 ${pid}`, { stdio: 'ignore' });
            console.log(`[clear-port] Killed process ${pid} on port ${PORT}`);
          }
        }
      } catch (e) {
        // 忽略未找到进程的错误
      }
    }
  } catch (error) {
    // 忽略清理阶段的一切非预期错误，保证主干执行
  }
}

// 等待端口完全释放（没有 LISTENING 状态）
function waitPortReleased(port, retries = 20, delay = 100) {
  return new Promise((resolve) => {
    let count = 0;
    const check = () => {
      let output = '';
      try {
        if (process.platform === 'win32') {
          output = execSync(`netstat -ano | findstr :${port}`, { encoding: 'utf8' });
        } else {
          output = execSync(`lsof -i:${port}`, { encoding: 'utf8' });
        }
      } catch (e) {}

      // 如果没有任何输出，或者输出中没有 LISTENING 状态，说明已经彻底释放
      if (!output || !output.includes('LISTENING')) {
        resolve(true);
      } else if (count < retries) {
        count++;
        setTimeout(check, delay);
      } else {
        // 超时直接继续，依靠 Vite 的自我检测
        resolve(false);
      }
    };
    check();
  });
}

async function start() {
  // 1. 清理后台进程与端口占用
  cleanProcesses();

  // 2. 等待操作系统彻底回收释放端口
  await waitPortReleased(PORT);

  // 3. 启动 Vite 开发服务器并代理其输出和退出码
  const child = spawn('node', ['node_modules/vite/bin/vite.js'], { stdio: 'inherit' });

  child.on('error', (err) => {
    console.error('[clear-port] Failed to start Vite:', err);
    process.exit(1);
  });

  child.on('exit', (code) => {
    process.exit(code || 0);
  });
}

start();
