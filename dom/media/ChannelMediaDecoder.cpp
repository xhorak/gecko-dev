/* -*- Mode: C++; tab-width: 2; indent-tabs-mode: nil; c-basic-offset: 2 -*- */
/* vim:set ts=2 sw=2 sts=2 et cindent: */
/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

#include "ChannelMediaDecoder.h"
#include "DecoderTraits.h"
#include "MediaDecoderStateMachine.h"
#include "MediaFormatReader.h"
#include "MediaResource.h"
#include "MediaShutdownManager.h"

namespace mozilla {

extern LazyLogModule gMediaDecoderLog;
#define LOG(x, ...)                                                            \
  MOZ_LOG(                                                                     \
    gMediaDecoderLog, LogLevel::Debug, ("Decoder=%p " x, this, ##__VA_ARGS__))

ChannelMediaDecoder::ResourceCallback::ResourceCallback(
  AbstractThread* aMainThread)
  : mAbstractMainThread(aMainThread)
{
  MOZ_ASSERT(aMainThread);
}

void
ChannelMediaDecoder::ResourceCallback::Connect(ChannelMediaDecoder* aDecoder)
{
  MOZ_ASSERT(NS_IsMainThread());
  mDecoder = aDecoder;
  mTimer = do_CreateInstance("@mozilla.org/timer;1");
  mTimer->SetTarget(mAbstractMainThread->AsEventTarget());
}

void
ChannelMediaDecoder::ResourceCallback::Disconnect()
{
  MOZ_ASSERT(NS_IsMainThread());
  if (mDecoder) {
    mDecoder = nullptr;
    mTimer->Cancel();
    mTimer = nullptr;
  }
}

MediaDecoderOwner*
ChannelMediaDecoder::ResourceCallback::GetMediaOwner() const
{
  MOZ_ASSERT(NS_IsMainThread());
  return mDecoder ? mDecoder->GetOwner() : nullptr;
}

void
ChannelMediaDecoder::ResourceCallback::NotifyNetworkError()
{
  MOZ_ASSERT(NS_IsMainThread());
  if (mDecoder) {
    mDecoder->NetworkError();
  }
}

/* static */ void
ChannelMediaDecoder::ResourceCallback::TimerCallback(nsITimer* aTimer,
                                                     void* aClosure)
{
  MOZ_ASSERT(NS_IsMainThread());
  ResourceCallback* thiz = static_cast<ResourceCallback*>(aClosure);
  MOZ_ASSERT(thiz->mDecoder);
  thiz->mDecoder->NotifyDataArrivedInternal();
  thiz->mTimerArmed = false;
}

void
ChannelMediaDecoder::ResourceCallback::NotifyDataArrived()
{
  MOZ_ASSERT(NS_IsMainThread());
  if (!mDecoder) {
    return;
  }

  mDecoder->DownloadProgressed();

  if (mTimerArmed) {
    return;
  }
  // In situations where these notifications come from stochastic network
  // activity, we can save significant computation by throttling the
  // calls to MediaDecoder::NotifyDataArrived() which will update the buffer
  // ranges of the reader.
  mTimerArmed = true;
  mTimer->InitWithNamedFuncCallback(
    TimerCallback, this, sDelay, nsITimer::TYPE_ONE_SHOT,
    "ChannelMediaDecoder::ResourceCallback::TimerCallback");
}

void
ChannelMediaDecoder::ResourceCallback::NotifyDataEnded(nsresult aStatus)
{
  RefPtr<ResourceCallback> self = this;
  nsCOMPtr<nsIRunnable> r = NS_NewRunnableFunction(
    "ChannelMediaDecoder::ResourceCallback::NotifyDataEnded",
    [=]() {
    if (!self->mDecoder) {
      return;
    }
    self->mDecoder->NotifyDownloadEnded(aStatus);
    if (NS_SUCCEEDED(aStatus)) {
      MediaDecoderOwner* owner = self->GetMediaOwner();
      MOZ_ASSERT(owner);
      owner->DownloadSuspended();

      // NotifySuspendedStatusChanged will tell the element that download
      // has been suspended "by the cache", which is true since we never
      // download anything. The element can then transition to HAVE_ENOUGH_DATA.
      self->mDecoder->NotifySuspendedStatusChanged();
    }
  });
  mAbstractMainThread->Dispatch(r.forget());
}

void
ChannelMediaDecoder::ResourceCallback::NotifyPrincipalChanged()
{
  MOZ_ASSERT(NS_IsMainThread());
  if (mDecoder) {
    mDecoder->NotifyPrincipalChanged();
  }
}

void
ChannelMediaDecoder::ResourceCallback::NotifySuspendedStatusChanged()
{
  MOZ_ASSERT(NS_IsMainThread());
  if (mDecoder) {
    mDecoder->NotifySuspendedStatusChanged();
  }
}

void
ChannelMediaDecoder::ResourceCallback::NotifyBytesConsumed(int64_t aBytes,
                                                           int64_t aOffset)
{
  RefPtr<ResourceCallback> self = this;
  nsCOMPtr<nsIRunnable> r = NS_NewRunnableFunction(
    "ChannelMediaDecoder::ResourceCallback::NotifyBytesConsumed",
    [=]() {
    if (self->mDecoder) {
      self->mDecoder->NotifyBytesConsumed(aBytes, aOffset);
    }
  });
  mAbstractMainThread->Dispatch(r.forget());
}

ChannelMediaDecoder::ChannelMediaDecoder(MediaDecoderInit& aInit)
  : MediaDecoder(aInit)
  , mResourceCallback(new ResourceCallback(aInit.mOwner->AbstractMainThread()))
  , mWatchManager(this, aInit.mOwner->AbstractMainThread())
{
  mResourceCallback->Connect(this);

  // mIgnoreProgressData
  mWatchManager.Watch(mLogicallySeeking, &ChannelMediaDecoder::SeekingChanged);
}

bool
ChannelMediaDecoder::CanClone()
{
  MOZ_ASSERT(NS_IsMainThread());
  return mResource && mResource->CanClone();
}

already_AddRefed<ChannelMediaDecoder>
ChannelMediaDecoder::Clone(MediaDecoderInit& aInit)
{
  if (!mResource) {
    return nullptr;
  }
  RefPtr<ChannelMediaDecoder> decoder = CloneImpl(aInit);
  if (!decoder) {
    return nullptr;
  }
  nsresult rv = decoder->Load(mResource);
  if (NS_FAILED(rv)) {
    decoder->Shutdown();
    return nullptr;
  }
  return decoder.forget();
}

MediaResource*
ChannelMediaDecoder::GetResource() const
{
  return mResource;
}

MediaDecoderStateMachine* ChannelMediaDecoder::CreateStateMachine()
{
  MOZ_ASSERT(NS_IsMainThread());
  MediaFormatReaderInit init;
  init.mVideoFrameContainer = GetVideoFrameContainer();
  init.mKnowsCompositor = GetCompositor();
  init.mCrashHelper = GetOwner()->CreateGMPCrashHelper();
  init.mFrameStats = mFrameStats;
  init.mResource = mResource;
  mReader = DecoderTraits::CreateReader(ContainerType(), init);
  return new MediaDecoderStateMachine(this, mReader);
}

void
ChannelMediaDecoder::Shutdown()
{
  mWatchManager.Shutdown();
  mResourceCallback->Disconnect();
  MediaDecoder::Shutdown();

  // Force any outstanding seek and byterange requests to complete
  // to prevent shutdown from deadlocking.
  if (mResource) {
    mResource->Close();
  }
}

nsresult
ChannelMediaDecoder::OpenResource(nsIStreamListener** aStreamListener)
{
  MOZ_ASSERT(NS_IsMainThread());
  if (aStreamListener) {
    *aStreamListener = nullptr;
  }
  return mResource->Open(aStreamListener);
}

nsresult
ChannelMediaDecoder::Load(nsIChannel* aChannel,
                          bool aIsPrivateBrowsing,
                          nsIStreamListener** aStreamListener)
{
  MOZ_ASSERT(NS_IsMainThread());
  MOZ_ASSERT(!mResource);

  mResource =
    BaseMediaResource::Create(mResourceCallback, aChannel, aIsPrivateBrowsing);
  if (!mResource) {
    return NS_ERROR_FAILURE;
  }

  nsresult rv = MediaShutdownManager::Instance().Register(this);
  if (NS_WARN_IF(NS_FAILED(rv))) {
    return rv;
  }

  rv = OpenResource(aStreamListener);
  NS_ENSURE_SUCCESS(rv, rv);

  SetStateMachine(CreateStateMachine());
  NS_ENSURE_TRUE(GetStateMachine(), NS_ERROR_FAILURE);

  return InitializeStateMachine();
}

nsresult
ChannelMediaDecoder::Load(BaseMediaResource* aOriginal)
{
  MOZ_ASSERT(NS_IsMainThread());
  MOZ_ASSERT(!mResource);

  mResource = aOriginal->CloneData(mResourceCallback);
  if (!mResource) {
    return NS_ERROR_FAILURE;
  }

  nsresult rv = MediaShutdownManager::Instance().Register(this);
  if (NS_WARN_IF(NS_FAILED(rv))) {
    return rv;
  }

  rv = OpenResource(nullptr);
  NS_ENSURE_SUCCESS(rv, rv);

  SetStateMachine(CreateStateMachine());
  NS_ENSURE_TRUE(GetStateMachine(), NS_ERROR_FAILURE);

  return InitializeStateMachine();
}

void
ChannelMediaDecoder::NotifyDownloadEnded(nsresult aStatus)
{
  MOZ_ASSERT(NS_IsMainThread());
  MOZ_DIAGNOSTIC_ASSERT(!IsShutdown());
  AbstractThread::AutoEnter context(AbstractMainThread());

  LOG("NotifyDownloadEnded, status=%" PRIx32, static_cast<uint32_t>(aStatus));

  if (aStatus == NS_BINDING_ABORTED) {
    // Download has been cancelled by user.
    GetOwner()->LoadAborted();
    return;
  }

  UpdatePlaybackRate();

  if (NS_SUCCEEDED(aStatus)) {
    // A final progress event will be fired by the MediaResource calling
    // DownloadSuspended on the element.
    // Also NotifySuspendedStatusChanged() will be called to update readyState
    // if download ended with success.
  } else if (aStatus != NS_BASE_STREAM_CLOSED) {
    NetworkError();
  }
}

void
ChannelMediaDecoder::NotifyBytesConsumed(int64_t aBytes, int64_t aOffset)
{
  MOZ_ASSERT(NS_IsMainThread());
  MOZ_DIAGNOSTIC_ASSERT(!IsShutdown());
  AbstractThread::AutoEnter context(AbstractMainThread());

  if (mIgnoreProgressData) {
    return;
  }

  MOZ_ASSERT(GetStateMachine());
  if (aOffset >= mDecoderPosition) {
    mPlaybackStatistics.AddBytes(aBytes);
  }
  mDecoderPosition = aOffset + aBytes;
}

void
ChannelMediaDecoder::SeekingChanged()
{
  // Stop updating the bytes downloaded for progress notifications when
  // seeking to prevent wild changes to the progress notification.
  MOZ_ASSERT(NS_IsMainThread());
  mIgnoreProgressData = mLogicallySeeking;
}

bool
ChannelMediaDecoder::CanPlayThroughImpl()
{
  MOZ_ASSERT(NS_IsMainThread());
  NS_ENSURE_TRUE(GetStateMachine(), false);
  return GetStatistics().CanPlayThrough();
}

void
ChannelMediaDecoder::OnPlaybackEvent(MediaEventType aEvent)
{
  MOZ_ASSERT(NS_IsMainThread());
  MediaDecoder::OnPlaybackEvent(aEvent);
  switch (aEvent) {
    case MediaEventType::PlaybackStarted:
      mPlaybackStatistics.Start();
      break;
    case MediaEventType::PlaybackStopped:
      mPlaybackStatistics.Stop();
      ComputePlaybackRate();
      break;
    default:
      break;
  }
}

void
ChannelMediaDecoder::DurationChanged()
{
  MOZ_ASSERT(NS_IsMainThread());
  AbstractThread::AutoEnter context(AbstractMainThread());
  MediaDecoder::DurationChanged();
  // Duration has changed so we should recompute playback rate
  UpdatePlaybackRate();
}

void
ChannelMediaDecoder::DownloadProgressed()
{
  MOZ_ASSERT(NS_IsMainThread());
  AbstractThread::AutoEnter context(AbstractMainThread());
  MediaDecoder::DownloadProgressed();
  UpdatePlaybackRate();
  mResource->ThrottleReadahead(ShouldThrottleDownload());
}

void
ChannelMediaDecoder::ComputePlaybackRate()
{
  MOZ_ASSERT(NS_IsMainThread());
  MOZ_ASSERT(mResource);

  int64_t length = mResource->GetLength();
  if (mozilla::IsFinite<double>(mDuration) && mDuration > 0 && length >= 0) {
    mPlaybackRateReliable = true;
    mPlaybackBytesPerSecond = length / mDuration;
    return;
  }

  bool reliable = false;
  mPlaybackBytesPerSecond = mPlaybackStatistics.GetRateAtLastStop(&reliable);
  mPlaybackRateReliable = reliable;
}

void
ChannelMediaDecoder::UpdatePlaybackRate()
{
  MOZ_ASSERT(NS_IsMainThread());
  MOZ_ASSERT(mResource);

  ComputePlaybackRate();
  uint32_t rate = mPlaybackBytesPerSecond;

  if (mPlaybackRateReliable) {
    // Avoid passing a zero rate
    rate = std::max(rate, 1u);
  } else {
    // Set a minimum rate of 10,000 bytes per second ... sometimes we just
    // don't have good data
    rate = std::max(rate, 10000u);
  }

  mResource->SetPlaybackRate(rate);
}

MediaStatistics
ChannelMediaDecoder::GetStatistics()
{
  MOZ_ASSERT(NS_IsMainThread());
  MOZ_ASSERT(mResource);

  MediaStatistics result;
  result.mDownloadRate =
    mResource->GetDownloadRate(&result.mDownloadRateReliable);
  result.mDownloadPosition = mResource->GetCachedDataEnd(mDecoderPosition);
  result.mTotalBytes = mResource->GetLength();
  result.mPlaybackRate = mPlaybackBytesPerSecond;
  result.mPlaybackRateReliable = mPlaybackRateReliable;
  result.mDecoderPosition = mDecoderPosition;
  result.mPlaybackPosition = mPlaybackPosition;
  return result;
}

bool
ChannelMediaDecoder::ShouldThrottleDownload()
{
  // We throttle the download if either the throttle override pref is set
  // (so that we can always throttle in Firefox on mobile) or if the download
  // is fast enough that there's no concern about playback being interrupted.
  MOZ_ASSERT(NS_IsMainThread());
  NS_ENSURE_TRUE(GetStateMachine(), false);

  int64_t length = mResource->GetLength();
  if (length > 0 &&
      length <= int64_t(MediaPrefs::MediaMemoryCacheMaxSize()) * 1024) {
    // Don't throttle the download of small resources. This is to speed
    // up seeking, as seeks into unbuffered ranges would require starting
    // up a new HTTP transaction, which adds latency.
    return false;
  }

  if (Preferences::GetBool("media.throttle-regardless-of-download-rate",
                           false)) {
    return true;
  }

  MediaStatistics stats = GetStatistics();
  if (!stats.mDownloadRateReliable || !stats.mPlaybackRateReliable) {
    return false;
  }
  uint32_t factor =
    std::max(2u, Preferences::GetUint("media.throttle-factor", 2));
  return stats.mDownloadRate > factor * stats.mPlaybackRate;
}

void
ChannelMediaDecoder::SetLoadInBackground(bool aLoadInBackground)
{
  MOZ_ASSERT(NS_IsMainThread());
  if (mResource) {
    mResource->SetLoadInBackground(aLoadInBackground);
  }
}

void
ChannelMediaDecoder::Suspend()
{
  MOZ_ASSERT(NS_IsMainThread());
  if (mResource) {
    mResource->Suspend(true);
  }
}

void
ChannelMediaDecoder::Resume()
{
  MOZ_ASSERT(NS_IsMainThread());
  if (mResource) {
    mResource->Resume();
  }
}

} // namespace mozilla

// avoid redefined macro in unified build
#undef LOG
