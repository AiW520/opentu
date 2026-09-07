import React, { useEffect, useMemo, useState } from 'react';
import { Search, RefreshCw, X } from 'lucide-react';
import { MessagePlugin } from 'tdesign-react';
import {
  recoverImageByRequestId,
  extractRequestId,
} from '../../services/media-api/image-api';
import {
  IMAGE_REQUEST_ID_EVENT,
  getLatestImageRequestId,
  setLatestImageRequestId,
} from '../../services/media-api/request-id-debug';
import { getAdapterContextFromSettings } from '../../services/model-adapters/context';
import type { ModelRef } from '../../utils/settings-manager';
import { RetryImage } from '../retry-image';
import './request-id-recovery-panel.scss';

interface RequestIdRecoveryPanelProps {
  selectedModel?: string;
  selectedModelRef?: ModelRef | null;
  language?: 'zh' | 'en';
  className?: string;
}

function getRecoverErrorMessage(
  error: unknown,
  language: 'zh' | 'en'
): string {
  const fallback =
    language === 'zh' ? '图片找回失败，请稍后再试' : 'Image recovery failed';

  if (!(error instanceof Error)) {
    return fallback;
  }

  const message = error.message || fallback;

  if (
    /找回接口返回状态非成功/.test(message) ||
    /processing_or_not_found/.test(message) ||
    /\bfailed\b/i.test(message)
  ) {
    return language === 'zh'
      ? '查找图片失败，没有成功生成图片'
      : 'Image lookup failed. The image was not generated successfully.';
  }

  return message;
}

export const RequestIdRecoveryPanel: React.FC<RequestIdRecoveryPanelProps> = ({
  selectedModel = '',
  selectedModelRef,
  language = 'zh',
  className,
}) => {
  const [requestId, setRequestId] = useState('');
  const [isRecovering, setIsRecovering] = useState(false);
  const [recoveredUrl, setRecoveredUrl] = useState('');
  const [recoveredError, setRecoveredError] = useState('');
  const [latestRequestId, setLatestRequestIdState] = useState(() =>
    getLatestImageRequestId()
  );

  useEffect(() => {
    const syncLatestRequestId = (event: Event) => {
      const nextRequestId =
        (event as CustomEvent<{ requestId?: string }>).detail?.requestId ||
        getLatestImageRequestId();
      setLatestRequestIdState(nextRequestId);
    };

    window.addEventListener(IMAGE_REQUEST_ID_EVENT, syncLatestRequestId);
    return () =>
      window.removeEventListener(IMAGE_REQUEST_ID_EVENT, syncLatestRequestId);
  }, []);

  const recoverConfig = useMemo(() => {
    const context = getAdapterContextFromSettings(
      'image',
      selectedModelRef || selectedModel || null
    );

    return {
      apiKey: context.provider?.apiKey || context.apiKey || '',
      baseUrl: context.provider?.baseUrl || context.baseUrl,
      authType: context.provider?.authType || context.authType,
      providerType: context.provider?.providerType,
      extraHeaders: context.provider?.extraHeaders || context.extraHeaders,
      provider: context.provider || null,
      binding: context.binding || null,
    };
  }, [selectedModel, selectedModelRef]);

  const handleUseLatest = () => {
    if (!latestRequestId) {
      MessagePlugin.warning(
        language === 'zh'
          ? '还没有最近一次 X-Request-Id，请先发起一次图片生成'
          : 'No recent X-Request-Id yet. Generate an image first.'
      );
      return;
    }

    setRequestId(extractRequestId(latestRequestId));
  };

  const handleRecover = async () => {
    const normalizedRequestId = extractRequestId(requestId);
    if (!normalizedRequestId) {
      MessagePlugin.warning(
        language === 'zh'
          ? '请输入 X-Request-Id'
          : 'Please enter an X-Request-Id'
      );
      return;
    }

    if (normalizedRequestId !== requestId) {
      setRequestId(normalizedRequestId);
    }

    setIsRecovering(true);
    setRecoveredError('');

    try {
      const recovered = await recoverImageByRequestId(
        normalizedRequestId,
        recoverConfig
      );
      setLatestImageRequestId(normalizedRequestId);
      setRecoveredUrl(recovered.url);
      MessagePlugin.success(
        language === 'zh' ? '图片找回成功' : 'Image recovered successfully'
      );
    } catch (error) {
      const message = getRecoverErrorMessage(error, language);
      setRecoveredUrl('');
      setRecoveredError(message);
      console.error(`[X-Request-Id][recover] ${normalizedRequestId} ${message}`);
      MessagePlugin.error(message);
    } finally {
      setIsRecovering(false);
    }
  };

  const handleCloseRecovered = () => {
    setRecoveredUrl('');
    setRecoveredError('');
  };

  return (
    <div className={`request-id-recovery ${className || ''}`.trim()}>
        <div className="request-id-recovery__header">
          <span className="request-id-recovery__title">X-Request-Id</span>
          <button
            type="button"
            className="request-id-recovery__chip"
            onClick={handleUseLatest}
          >
            <RefreshCw size={12} />
            <span>{language === 'zh' ? '填入最近一次' : 'Use latest'}</span>
          </button>
        </div>

        <div className="request-id-recovery__controls">
          <input
            className="request-id-recovery__input"
            value={requestId}
            onChange={(event) => {
              setRequestId(event.target.value);
              if (recoveredError) {
                setRecoveredError('');
              }
            }}
            onKeyDown={(event) => {
              if (event.key === 'Enter' && !isRecovering) {
                event.preventDefault();
                handleRecover();
              }
            }}
            placeholder={
              language === 'zh'
                ? '输入生成后输出到控制台的 X-Request-Id'
                : 'Paste the X-Request-Id printed in the console'
            }
          />
          <button
            type="button"
            className="request-id-recovery__action"
            onClick={handleRecover}
            disabled={isRecovering}
          >
            <Search size={14} />
            <span>{language === 'zh' ? '查询' : 'Recover'}</span>
          </button>
        </div>

        {latestRequestId ? (
          <div className="request-id-recovery__status">
            {language === 'zh' ? '最近一次' : 'Latest'}: {latestRequestId}
          </div>
        ) : null}

        {recoveredUrl ? (
          <div className="request-id-recovery__result">
            <div className="request-id-recovery__preview-shell">
              <RetryImage
                src={recoveredUrl}
                alt="Recovered image"
                className="request-id-recovery__preview"
                wrapperClassName="request-id-recovery__preview-wrap"
                showSkeleton={false}
              />
              <button
                type="button"
                className="request-id-recovery__close"
                onClick={handleCloseRecovered}
                aria-label={
                  language === 'zh' ? '关闭找回图片' : 'Close recovered image'
                }
              >
                <X size={12} />
              </button>
            </div>
            <div className="request-id-recovery__meta">
              <span className="request-id-recovery__status request-id-recovery__status--success">
                {language === 'zh' ? '找回成功' : 'Recovered'}
              </span>
              <span className="request-id-recovery__caption">
                {language === 'zh'
                  ? '图片已找回，可在这里临时查看'
                  : 'Recovered image preview is available here'}
              </span>
              <a
                className="request-id-recovery__link"
                href={recoveredUrl}
                target="_blank"
                rel="noreferrer"
              >
                {language === 'zh' ? '打开原图' : 'Open image'}
              </a>
            </div>
          </div>
        ) : null}

        {!recoveredUrl && recoveredError ? (
          <div className="request-id-recovery__status request-id-recovery__status--error">
            {recoveredError}
          </div>
        ) : null}
    </div>
  );
};

export default RequestIdRecoveryPanel;
