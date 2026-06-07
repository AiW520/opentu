import '../web/src/utils/permissions-policy-fix';
import { isTauriEnvironment } from './utils/tauri-api';
import { initializeVirtualUrlInterceptor } from './utils/virtual-url-interceptor';

function updateBootProgress(progress: number) {
  const fill = document.getElementById('boot-progress-fill');
  const value = document.getElementById('boot-progress-value');
  if (fill) fill.style.width = progress + '%';
  if (value) value.textContent = progress + '%';
}

function hideBootScreen() {
  const bootRoot = document.getElementById('app-boot-loading');
  if (bootRoot) {
    bootRoot.classList.add('is-leaving');
    setTimeout(() => {
      if (bootRoot.parentNode) {
        bootRoot.parentNode.removeChild(bootRoot);
      }
    }, 360);
  }
}

// 立即初始化虚拟 URL 拦截器（在应用启动前）
if (isTauriEnvironment()) {
  console.log('[Desktop] Early initialize virtual URL interceptor');
  initializeVirtualUrlInterceptor();
}

async function bootstrap() {
  updateBootProgress(30);

  // 初始化保存位置函数（仅在桌面环境中）
  if (isTauriEnvironment()) {
    const { writeFile } = await import('@tauri-apps/plugin-fs');
    // 动态设置保存函数
    const { setSaveLocationFn } = await import('@drawnix/drawnix');
    setSaveLocationFn(async (path: string, data: Uint8Array) => {
      await writeFile(path, data);
    });
  }

  updateBootProgress(60);

  import('../web/src/app/bootstrap').then(() => {
    updateBootProgress(100);

    // 再次确保拦截器已初始化（兜底）
    if (isTauriEnvironment()) {
      initializeVirtualUrlInterceptor();
    }

    setTimeout(hideBootScreen, 200);
  }).catch((error) => {
    console.error('[Desktop] Failed to load app bootstrap:', error);
    updateBootProgress(100);
    const tip = document.querySelector('.app-boot-tip');
    if (tip) tip.textContent = '启动失败，请重启应用';
  });
}

bootstrap();