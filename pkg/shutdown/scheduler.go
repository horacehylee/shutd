package shutdown

import (
	"fmt"
	"time"

	"github.com/go-co-op/gocron"
	"github.com/sirupsen/logrus"
)

const shutdownTag = "shutdown"
const snoozeNotificationTag = "snoozeNotification"

type scheduler struct {
	*gocron.Scheduler
	logger                *logrus.Logger
	config                Config
	shutdownJob           *gocron.Job
	snoozeNotificationJob *gocron.Job
}

type option func(*scheduler)

func WithLogger(logger *logrus.Logger) option {
	return func(s *scheduler) {
		s.logger = logger
	}
}

func NewScheduler(config Config, options ...option) (*scheduler, error) {
	s := gocron.NewScheduler(time.Local)
	s.StartAsync()
	s.TagsUnique()

	scheduler := &scheduler{
		Scheduler: s,
		logger:    logrus.New(),
	}
	for _, o := range options {
		o(scheduler)
	}
	err := scheduler.Configure(config)
	if err != nil {
		return nil, err
	}
	return scheduler, nil
}

func (s *scheduler) Configure(config Config) error {
	s.config = config
	s.logger.Infof("config: %+v", config)

	var err error
	err = s.scheduleShutdownJob(s.config.StartTime)
	if err != nil {
		return err
	}
	err = s.scheduleSnoozeNotificationJob()
	if err != nil {
		return err
	}

	s.printJobs()
	return nil
}

func (s *scheduler) scheduleShutdownJob(shutdownTime interface{}) error {
	if s.shutdownJob == nil {
		j, err := s.Every(1).Day().At(shutdownTime).Tag(shutdownTag).Do(s.newShutdownTask())
		if err != nil {
			// not wrapping error to expose implementation details
			return fmt.Errorf("failed to schedule shutdown job: %v", err)
		}
		s.shutdownJob = j
	} else {
		_, err := s.Job(s.shutdownJob).At(shutdownTime).Update()
		if err != nil {
			// not wrapping error to expose implementation details
			return fmt.Errorf("failed to update scheduled shutdown job: %v", err)
		}
	}
	return nil
}

func (s *scheduler) scheduleSnoozeNotificationJob() error {
	if s.shutdownJob == nil {
		return fmt.Errorf("shutdown job is not scheduled yet")
	}

	notifyTime := s.shutdownJob.ScheduledTime().Add(-time.Duration(s.config.Notification.Before) * time.Minute)
	if s.snoozeNotificationJob == nil {
		s.Every(1).Day().At(notifyTime).Tag(snoozeNotificationTag)
		if time.Now().After(notifyTime) {
			s.StartImmediately()
		}
		j, err := s.Do(s.newSnoozeNotificationTask())
		if err != nil {
			// not wrapping error to expose implementation details
			return fmt.Errorf("failed to schedule snooze notification job: %v", err)
		}
		s.snoozeNotificationJob = j
	} else {
		s.Job(s.snoozeNotificationJob).At(notifyTime)
		if time.Now().After(notifyTime) {
			s.StartImmediately()
		}
		_, err := s.Update()
		if err != nil {
			// not wrapping error to expose implementation details
			return fmt.Errorf("failed to update scheduled snooze notification job: %v", err)
		}
	}
	return nil
}

func (s *scheduler) snooze(interval int) error {
	if s.shutdownJob == nil {
		return fmt.Errorf("shutdown job is not scheduled")
	}
	var err error

	delayedTime := s.shutdownJob.ScheduledTime().Add(time.Duration(interval) * time.Minute)
	err = s.scheduleShutdownJob(delayedTime)
	if err != nil {
		return err
	}
	err = s.scheduleSnoozeNotificationJob()
	if err != nil {
		return err
	}

	s.printJobs()
	return nil
}

func (s *scheduler) printJobs() {
	for _, j := range s.Jobs() {
		s.logger.Infof("job: %v, scheduled: %v (%v)", j.Tags(), j.ScheduledAtTime(), j.ScheduledTime())
	}
}
